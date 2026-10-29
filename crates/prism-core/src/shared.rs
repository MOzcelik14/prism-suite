// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Suite-wide preferences shared between the launcher and creative applications (triM., steM., beaM.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedPreferences {
    /// Active UI language code ("en", "tr").
    pub language: String,
    /// Current launcher version.
    pub launcher_version: String,
}

impl Default for SharedPreferences {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl SharedPreferences {
    /// Returns the standard shared preferences file location `<config_dir>/prism/shared.ron`.
    pub fn file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "prism", "prism")
            .or_else(|| ProjectDirs::from("com", "prism", "prism-suite"))
            .map(|proj| proj.config_dir().join("shared.ron"))
    }

    /// Loads shared preferences from disk or creates default.
    pub fn load() -> Self {
        if let Some(path) = Self::file_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(prefs) = ron::from_str(&content) {
                        return prefs;
                    }
                }
            }
        }
        Self::default()
    }

    /// Writes shared preferences to disk.
    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::file_path() {
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
    fn test_shared_preferences_serialization() {
        let prefs = SharedPreferences {
            language: "tr".to_string(),
            launcher_version: "0.1.0".to_string(),
        };

        let ron_str = ron::to_string(&prefs).unwrap();
        assert!(ron_str.contains("language") && ron_str.contains("\"tr\""));

        let deserialized: SharedPreferences = ron::from_str(&ron_str).unwrap();
        assert_eq!(prefs, deserialized);
    }
}
