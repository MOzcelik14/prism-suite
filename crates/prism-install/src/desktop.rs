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
fn linux_applications_dir() -> Result<PathBuf, DesktopError> {
    let base = BaseDirs::new().ok_or(DesktopError::NoUserDirs)?;
    Ok(base.data_local_dir().join("applications"))
}

#[cfg(target_os = "linux")]
fn install_linux_desktop_file(app: &AppEntry, binary_path: &Path) -> Result<(), DesktopError> {
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
         Terminal=false\n\
         Categories={}\n\
         StartupNotify=true\n",
        app.display_name,
        app.category,
        binary_path.display(),
        category_str
    );

    fs::write(desktop_file_path, content)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn remove_linux_desktop_file(app: &AppEntry) -> Result<(), DesktopError> {
    let apps_dir = linux_applications_dir()?;
    let desktop_file_path = apps_dir.join(format!("prism-{}.desktop", app.id.as_str()));
    if desktop_file_path.exists() {
        let _ = fs::remove_file(desktop_file_path);
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
mod tests {
    use super::*;
    use prism_core::catalog::AppCatalog;

    #[test]
    fn test_linux_desktop_entry_generation() {
        let catalog = AppCatalog::load_embedded();
        let trim = catalog.get(prism_core::AppId::Trim).unwrap();
        let bin_path = Path::new("/opt/prism/apps/trim/current/trim");

        #[cfg(target_os = "linux")]
        {
            let res = install_linux_desktop_file(trim, bin_path);
            assert!(res.is_ok());

            let apps_dir = linux_applications_dir().unwrap();
            let file_path = apps_dir.join("prism-trim.desktop");
            assert!(file_path.exists());

            let content = fs::read_to_string(&file_path).unwrap();
            assert!(content.contains("Name=triM."));
            assert!(content.contains("Exec=\"/opt/prism/apps/trim/current/trim\" %F"));

            let _ = remove_linux_desktop_file(trim);
            assert!(!file_path.exists());
        }
    }
}
