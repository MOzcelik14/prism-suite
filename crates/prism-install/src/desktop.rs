// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

#[cfg(target_os = "linux")]
use std::fs;
use std::path::Path;
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
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

/// Generates and installs desktop integration (.desktop file on Linux, shortcut on Windows).
pub fn install_desktop_entry(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
    #[cfg(target_os = "linux")]
    {
        install_linux_desktop_file(app, binary_path)?;
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

#[cfg(target_os = "windows")]
fn install_windows_shortcut(_app: &AppEntry, _binary_path: &Path) -> Result<(), DesktopError> {
    // Windows shortcut generation stub
    Ok(())
}

#[cfg(target_os = "windows")]
fn remove_windows_shortcut(_app: &AppEntry) -> Result<(), DesktopError> {
    // Windows shortcut cleanup stub
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
