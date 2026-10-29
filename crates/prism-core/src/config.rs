// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use crate::state::ReleaseChannel;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Update check frequency preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateFrequency {
    #[default]
    EveryLaunch,
    Daily,
    Weekly,
    Manual,
}

impl UpdateFrequency {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateFrequency::EveryLaunch => "every_launch",
            UpdateFrequency::Daily => "daily",
            UpdateFrequency::Weekly => "weekly",
            UpdateFrequency::Manual => "manual",
        }
    }
}

/// Persistent configuration for the prisM. Suite launcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherConfig {
    /// Preferred UI language (e.g. "en", "tr"). If None, detects from system.
    pub language: Option<String>,
    /// Selected update release channel.
    pub release_channel: ReleaseChannel,
    /// Number of previous versions to retain for rollback (default: 2, range 1..=5).
    pub keep_versions_count: usize,
    /// Custom root installation directory for apps. Defaults to system app data directory.
    pub custom_install_dir: Option<PathBuf>,
    /// Automatically check for updates on startup.
    pub check_updates_on_launch: bool,
    /// Frequency at which update checks should run.
    pub check_frequency: UpdateFrequency,
    /// Automatically download and apply updates without prompting.
    pub auto_update: bool,
    /// Timestamp (seconds since unix epoch) of the last update check.
    pub last_update_check: Option<u64>,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            language: None,
            release_channel: ReleaseChannel::default(),
            keep_versions_count: 2,
            custom_install_dir: None,
            check_updates_on_launch: true,
            check_frequency: UpdateFrequency::default(),
            auto_update: false,
            last_update_check: None,
        }
    }
}

impl LauncherConfig {
    /// Resets all configuration settings to default values.
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    /// Checks whether an update check should be performed on startup based on check_frequency and last_update_check.
    pub fn should_check_on_startup(&self) -> bool {
        if !self.check_updates_on_launch {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        match self.check_frequency {
            UpdateFrequency::EveryLaunch => true,
            UpdateFrequency::Daily => match self.last_update_check {
                Some(last) => now.saturating_sub(last) >= 86400,
                None => true,
            },
            UpdateFrequency::Weekly => match self.last_update_check {
                Some(last) => now.saturating_sub(last) >= 604800,
                None => true,
            },
            UpdateFrequency::Manual => false,
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launcher_config_serialization_and_defaults() {
        let mut config = LauncherConfig {
            language: Some("tr".to_string()),
            release_channel: ReleaseChannel::Beta,
            keep_versions_count: 4,
            check_frequency: UpdateFrequency::Weekly,
            auto_update: true,
            ..Default::default()
        };

        let ron_str = ron::to_string(&config).unwrap();
        let loaded: LauncherConfig = ron::from_str(&ron_str).unwrap();
        assert_eq!(config, loaded);

        config.reset_to_defaults();
        assert_eq!(config.language, None);
        assert_eq!(config.release_channel, ReleaseChannel::Stable);
        assert_eq!(config.keep_versions_count, 2);
        assert_eq!(config.check_frequency, UpdateFrequency::EveryLaunch);
        assert!(!config.auto_update);
    }
}
