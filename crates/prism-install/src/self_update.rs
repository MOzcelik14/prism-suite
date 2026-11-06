// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use crate::archive::extract_archive;
use crate::InstallError;
use std::fs;
use std::path::{Path, PathBuf};

/// Finds the launcher executable file within an extracted directory.
fn find_launcher_binary(dir: &Path) -> Result<PathBuf, InstallError> {
    #[cfg(windows)]
    let expected_names = ["prism.exe", "prism-suite.exe"];
    #[cfg(not(windows))]
    let expected_names = ["prism", "prism-suite"];

    // 1. Direct match in root
    for name in &expected_names {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }

    // 2. Recursive search
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(found) = find_launcher_binary(&path) {
                return Ok(found);
            }
        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if expected_names.contains(&file_name) {
                return Ok(path);
            }
        }
    }

    Err(InstallError::ExecutableNotFound(
        dir.join(expected_names[0]),
    ))
}

/// Applies self-update by safely extracting the updated archive and replacing current_exe.
pub fn apply_self_update(
    archive_filename: &str,
    archive_bytes: &[u8],
    current_exe: &Path,
) -> Result<(), InstallError> {
    let rand_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_dir = std::env::temp_dir().join(format!("prism-self-update-{}", rand_id));
    fs::create_dir_all(&temp_dir)?;

    // Safe extraction into temp staging
    extract_archive(archive_filename, archive_bytes, &temp_dir)?;

    let new_binary = find_launcher_binary(&temp_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&new_binary)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&new_binary, perms)?;

        // Check if the current executable's directory is writable by the current user.
        // System packages (e.g. /usr/bin/prism) or AppImage squashfs mounts are read-only for normal users.
        let is_writable = current_exe
            .parent()
            .map(|p| {
                let test_path = p.join(format!(".prism_write_test_{}", rand_id));
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&test_path)
                {
                    Ok(_) => {
                        let _ = fs::remove_file(&test_path);
                        true
                    }
                    Err(_) => false,
                }
            })
            .unwrap_or(false);

        let target_exe = if is_writable {
            current_exe.to_path_buf()
        } else if let Some(base) = directories::BaseDirs::new() {
            let user_bin = base.home_dir().join(".local/bin");
            fs::create_dir_all(&user_bin)?;
            let bin_name = current_exe
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("prism"));
            let user_target = user_bin.join(bin_name);

            // Install or override user desktop file so the desktop environment launches the updated user binary
            let apps_dir = base.data_local_dir().join("applications");
            if fs::create_dir_all(&apps_dir).is_ok() {
                let desktop_file = apps_dir.join("prism.desktop");
                let desktop_content = format!(
                    "[Desktop Entry]\n\
                    Name=prisM. Suite\n\
                    GenericName=Creative Suite Launcher\n\
                    GenericName[tr]=Yaratıcı Yazılım Paketi Başlatıcı\n\
                    Comment=Unified desktop launcher, installer, and auto-updater for triM., steM., and beaM.\n\
                    Comment[tr]=triM., steM. ve beaM. için merkezi masaüstü başlatıcı ve güncelleme yöneticisi\n\
                    Exec=\"{exec}\" %F\n\
                    Icon=prism\n\
                    Terminal=false\n\
                    Type=Application\n\
                    Categories=Utility;Development;Graphics;AudioVideo;\n\
                    StartupNotify=true\n\
                    StartupWMClass=prism\n\
                    Keywords=launcher;updater;suite;creative;trim;stem;beam;store;\n",
                    exec = user_target.display()
                );
                let _ = fs::write(desktop_file, desktop_content);
            }
            user_target
        } else {
            current_exe.to_path_buf()
        };

        // On Unix, atomic rename replaces the active running binary without ETXTBSY.
        // If rename fails (e.g. cross-device EXDEV), fall back to copy and remove.
        let backup_path = PathBuf::from(format!("{}.old", target_exe.display()));
        let _ = fs::remove_file(&backup_path);

        if target_exe.exists() {
            let _ = fs::rename(&target_exe, &backup_path);
        }

        let replace_res = if fs::rename(&new_binary, &target_exe).is_err() {
            match fs::copy(&new_binary, &target_exe) {
                Ok(_) => {
                    let _ = fs::remove_file(&new_binary);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(())
        };

        if let Err(e) = replace_res {
            // Rollback to backup binary if replacement failed
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &target_exe);
            }
            return Err(InstallError::Io(e));
        }

        let _ = fs::remove_file(&backup_path);
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[cfg(windows)]
    {
        // On Windows, running binaries are file-locked; staging & helper script handles post-exit replacement
        let new_target = PathBuf::from(format!("{}.new", current_exe.display()));
        let _ = fs::remove_file(&new_target);
        fs::copy(&new_binary, &new_target)?;

        let bat_content = format!(
            "@echo off\r\n\
            :wait\r\n\
            timeout /t 1 /nobreak >nul\r\n\
            tasklist /fi \"PID eq {pid}\" | find \"{pid}\" >nul\r\n\
            if not errorlevel 1 goto wait\r\n\
            move /y \"{new_exe}\" \"{target_exe}\" >nul\r\n\
            start \"\" \"{target_exe}\"\r\n\
            del \"%~f0\" & exit\r\n",
            pid = std::process::id(),
            new_exe = new_target.display(),
            target_exe = current_exe.display(),
        );

        let bat_path = std::env::temp_dir().join(format!("prism-update-helper-{}.bat", rand_id));
        fs::write(&bat_path, bat_content)?;

        let _ = crate::process::launch_detached(&bat_path, std::env::temp_dir().as_path());
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_apply_self_update_linux_mock() {
        let temp_dir = std::env::temp_dir().join("prism_test_self_update");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let mock_current_exe = temp_dir.join("prism");
        fs::write(&mock_current_exe, b"old_binary_contents").unwrap();

        // Create a tar.gz with updated prism binary
        let mut tar_builder = tar::Builder::new(Vec::new());
        let updated_contents = b"new_updated_binary_contents";
        let mut header = tar::Header::new_gnu();
        header.set_path("prism").unwrap();
        header.set_size(updated_contents.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar_builder.append(&header, &updated_contents[..]).unwrap();
        let tar_bytes = tar_builder.into_inner().unwrap();

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&tar_bytes).unwrap();
        let gz_bytes = encoder.finish().unwrap();

        apply_self_update(
            "prism-0.2.0-linux-x86_64.tar.gz",
            &gz_bytes,
            &mock_current_exe,
        )
        .unwrap();

        let read_back = fs::read(&mock_current_exe).unwrap();
        assert_eq!(read_back, updated_contents);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
