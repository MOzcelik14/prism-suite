use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Persistent configuration for the prisM. Suite launcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    /// Preferred UI language (e.g. "en", "tr"). If None, detects from system.
    pub language: Option<String>,
    /// Custom root installation directory for apps. Defaults to system app data directory.
    pub custom_install_dir: Option<PathBuf>,
    /// Automatically check for updates on startup.
    pub check_updates_on_launch: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            language: None,
            custom_install_dir: None,
            check_updates_on_launch: true,
        }
    }
}

impl LauncherConfig {
    /// Locates the configuration directory.
    pub fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "prism", "prism-suite")
            .map(|proj| proj.config_dir().join("config.ron"))
    }

    /// Loads the configuration from disk, returning default if not found.
    pub fn load() -> Self {
        if let Some(path) = Self::config_file_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = ron::from_str::<Self>(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    /// Saves the current configuration to disk.
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_file_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let serialized = ron::to_string(self).map_err(std::io::Error::other)?;
            fs::write(path, serialized)?;
        }
        Ok(())
    }
}
