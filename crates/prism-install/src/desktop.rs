// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use prism_core::catalog::AppEntry;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DesktopError {
    #[error("I/O error during desktop integration: {0}")]
    Io(#[from] std::io::Error),

    #[error("Could not determine user directories for shortcuts")]
    NoUserDirs,
}

/// Generates and installs desktop integration (.desktop file on Linux, .app bundle on macOS, Start Menu shortcut on Windows).
pub fn install_desktop_entry(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
    #[cfg(target_os = "linux")]
    {
        install_linux_desktop_file(app, binary_path)?;
    }
    #[cfg(target_os = "macos")]
    {
        install_macos_app_bundle(app, binary_path)?;
    }
    #[cfg(target_os = "windows")]
    {
        install_windows_shortcut(app, binary_path)?;
    }
    Ok(())
}

/// Removes desktop integration for an application.
pub fn remove_desktop_entry(app: &AppEntry) -> Result<(), DesktopError> {
    #[cfg(target_os = "linux")]
    {
        remove_linux_desktop_file(app)?;
    }
    #[cfg(target_os = "macos")]
    {
        remove_macos_app_bundle(app)?;
    }
    #[cfg(target_os = "windows")]
    {
        remove_windows_shortcut(app)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn get_app_icon_png(app_id: prism_core::AppId) -> &'static [u8] {
    match app_id {
        prism_core::AppId::Trim => include_bytes!("../../../assets/apps/trim.png"),
        prism_core::AppId::Stem => include_bytes!("../../../assets/apps/stem.png"),
        prism_core::AppId::Beam => include_bytes!("../../../assets/apps/beam.png"),
        prism_core::AppId::Dram => include_bytes!("../../../assets/apps/dram.png"),
    }
}

#[cfg(target_os = "linux")]
fn linux_applications_dir() -> Result<PathBuf, DesktopError> {
    let base = BaseDirs::new().ok_or(DesktopError::NoUserDirs)?;
    Ok(base.data_local_dir().join("applications"))
}

#[cfg(target_os = "linux")]
fn linux_icon_paths(app_id: prism_core::AppId) -> Result<(PathBuf, PathBuf), DesktopError> {
    let base = BaseDirs::new().ok_or(DesktopError::NoUserDirs)?;
    let hicolor_dir = base
        .data_local_dir()
        .join("icons")
        .join("hicolor")
        .join("256x256")
        .join("apps");
    let pixmaps_dir = base.data_local_dir().join("pixmaps");
    let filename = format!("prism-{}.png", app_id.as_str());
    Ok((hicolor_dir.join(&filename), pixmaps_dir.join(&filename)))
}

#[cfg(target_os = "linux")]
fn install_linux_desktop_file(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
    use std::os::unix::fs::PermissionsExt;

    // Ensure target binary is executable if it exists
    if binary_path.exists() {
        let _ = fs::set_permissions(binary_path, fs::Permissions::from_mode(0o755));
    }

    // Determine working directory (parent of binary)
    let working_dir = binary_path.parent().unwrap_or_else(|| Path::new("/"));

    // Install icon to both hicolor theme and pixmaps
    let (hicolor_icon, pixmaps_icon) = linux_icon_paths(app.id)?;
    if let Some(parent) = hicolor_icon.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = pixmaps_icon.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let icon_bytes = get_app_icon_png(app.id);
    let _ = fs::write(&hicolor_icon, icon_bytes);
    let _ = fs::write(&pixmaps_icon, icon_bytes);

    // Prepare desktop file
    let apps_dir = linux_applications_dir()?;
    fs::create_dir_all(&apps_dir)?;

    let desktop_file_path = apps_dir.join(format!("prism-{}.desktop", app.id.as_str()));

    let category_str = match app.category.as_str() {
        "video" => "AudioVideo;Video;AudioVideoEditing;",
        "audio" => "AudioVideo;Audio;AudioVideoEditing;",
        "photo" => "Graphics;Photography;RasterGraphics;",
        _ => "Utility;",
    };

    let content = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={}\n\
         GenericName={}\n\
         Comment=Part of prisM. Suite\n\
         Exec=\"{}\" %F\n\
         Path={}\n\
         Icon={}\n\
         Terminal=false\n\
         Categories={}\n\
         StartupNotify=true\n\
         StartupWMClass={}\n",
        app.display_name,
        app.category,
        binary_path.display(),
        working_dir.display(),
        hicolor_icon.display(),
        category_str,
        app.primary_binary_name
    );

    fs::write(&desktop_file_path, &content)?;
    let _ = fs::set_permissions(&desktop_file_path, fs::Permissions::from_mode(0o755));

    // Also update ~/Desktop/prism-{id}.desktop if it already exists
    if let Some(base) = BaseDirs::new() {
        let user_desktop = base.home_dir().join("Desktop");
        let desktop_shortcut = user_desktop.join(format!("prism-{}.desktop", app.id.as_str()));
        if desktop_shortcut.exists() {
            let _ = fs::write(&desktop_shortcut, &content);
            let _ = fs::set_permissions(&desktop_shortcut, fs::Permissions::from_mode(0o755));
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_linux_desktop_file(app: &AppEntry) -> Result<(), DesktopError> {
    let apps_dir = linux_applications_dir()?;
    let desktop_file_path = apps_dir.join(format!("prism-{}.desktop", app.id.as_str()));
    if desktop_file_path.exists() {
        let _ = fs::remove_file(desktop_file_path);
    }

    if let Some(base) = BaseDirs::new() {
        let user_desktop = base.home_dir().join("Desktop");
        let desktop_shortcut = user_desktop.join(format!("prism-{}.desktop", app.id.as_str()));
        if desktop_shortcut.exists() {
            let _ = fs::remove_file(desktop_shortcut);
        }
    }

    if let Ok((hicolor_icon, pixmaps_icon)) = linux_icon_paths(app.id) {
        if hicolor_icon.exists() {
            let _ = fs::remove_file(hicolor_icon);
        }
        if pixmaps_icon.exists() {
            let _ = fs::remove_file(pixmaps_icon);
        }
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_app_bundle_path(app: &AppEntry) -> Result<PathBuf, DesktopError> {
    let base = BaseDirs::new().ok_or(DesktopError::NoUserDirs)?;
    let clean_name = app.display_name.trim_end_matches('.');
    Ok(base
        .home_dir()
        .join("Applications")
        .join("prisM. Suite")
        .join(format!("{clean_name}.app")))
}

#[cfg(target_os = "macos")]
fn install_macos_app_bundle(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
    use std::os::unix::fs::PermissionsExt;

    let bundle_dir = macos_app_bundle_path(app)?;
    let macos_dir = bundle_dir.join("Contents").join("MacOS");
    fs::create_dir_all(&macos_dir)?;

    let exec_name = &app.primary_binary_name;
    let launcher_path = macos_dir.join(exec_name);
    let quoted_bin = binary_path
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let script = format!("#!/bin/sh\nexec \"{quoted_bin}\" \"$@\"\n");
    fs::write(&launcher_path, script)?;

    let mut perms = fs::metadata(&launcher_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&launcher_path, perms)?;

    let bundle_name = app.display_name.trim_end_matches('.');
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>{exec_name}</string>
  <key>CFBundleIdentifier</key><string>com.mozcelik14.prism.{id}</string>
  <key>CFBundleName</key><string>{bundle_name}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#,
        id = app.id.as_str()
    );
    fs::write(bundle_dir.join("Contents").join("Info.plist"), plist)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn remove_macos_app_bundle(app: &AppEntry) -> Result<(), DesktopError> {
    if let Ok(bundle_dir) = macos_app_bundle_path(app) {
        if bundle_dir.exists() {
            fs::remove_dir_all(&bundle_dir)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_shortcut_path(app: &AppEntry) -> Result<PathBuf, DesktopError> {
    let base = BaseDirs::new().ok_or(DesktopError::NoUserDirs)?;
    let clean_name = app.display_name.trim_end_matches('.');
    Ok(base
        .data_dir()
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("prisM. Suite")
        .join(format!("{clean_name}.lnk")))
}

#[cfg(target_os = "windows")]
fn ps_single_quote(s: &str) -> String {
    s.replace('\'', "''")
}

#[cfg(target_os = "windows")]
fn install_windows_shortcut(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let shortcut_path = windows_shortcut_path(app)?;
    if let Some(parent) = shortcut_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let work_dir = binary_path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.WorkingDirectory = '{}'; \
         $s.Save()",
        ps_single_quote(&shortcut_path.to_string_lossy()),
        ps_single_quote(&binary_path.to_string_lossy()),
        ps_single_quote(&work_dir),
    );

    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if !status.success() {
        return Err(DesktopError::Io(std::io::Error::other(format!(
            "powershell shortcut creation exited with {status}"
        ))));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_windows_shortcut(app: &AppEntry) -> Result<(), DesktopError> {
    if let Ok(shortcut_path) = windows_shortcut_path(app) {
        if shortcut_path.exists() {
            fs::remove_file(&shortcut_path)?;
        }
        if let Some(parent) = shortcut_path.parent() {
            let _ = fs::remove_dir(parent);
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;
    use prism_core::catalog::AppCatalog;

    #[test]
    fn test_linux_desktop_entry_generation() {
        let catalog = AppCatalog::load_embedded();
        let trim = catalog.get(prism_core::AppId::Trim).unwrap();
        let bin_path = Path::new("/opt/prism/apps/trim/current/trim");

        let res = install_linux_desktop_file(trim, bin_path);
        assert!(res.is_ok());

        let apps_dir = linux_applications_dir().unwrap();
        let file_path = apps_dir.join("prism-trim.desktop");
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Name=triM."));
        assert!(content.contains("Exec=\"/opt/prism/apps/trim/current/trim\" %F"));
        assert!(content.contains("Path=/opt/prism/apps/trim/current"));
        assert!(content.contains("Icon="));
        assert!(content.contains("prism-trim.png"));
        assert!(content.contains("StartupWMClass=trim"));

        let (hicolor_icon, pixmaps_icon) = linux_icon_paths(trim.id).unwrap();
        assert!(hicolor_icon.exists());
        assert!(pixmaps_icon.exists());

        let _ = remove_linux_desktop_file(trim);
        assert!(!file_path.exists());
        assert!(!hicolor_icon.exists());
        assert!(!pixmaps_icon.exists());
    }
}
