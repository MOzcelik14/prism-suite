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

        // On Unix, atomic rename replaces the active running binary without ETXTBSY
        let backup_path = PathBuf::from(format!("{}.old", current_exe.display()));
        let _ = fs::remove_file(&backup_path);

        if current_exe.exists() {
            fs::rename(current_exe, &backup_path)?;
        }

        fs::rename(&new_binary, current_exe)?;
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
