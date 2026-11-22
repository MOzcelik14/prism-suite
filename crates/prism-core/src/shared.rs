// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use crate::catalog::AppId;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT_PROJECTS: usize = 20;

/// Metadata for a recently opened creative project across the prisM. Suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentProject {
    pub app_id: AppId,
    pub path: PathBuf,
    pub name: String,
    pub last_opened_unix: u64,
}

/// Suite-wide preferences shared between the launcher and creative applications (triM., steM., beaM., draM.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SharedPreferences {
    /// Active UI language code ("en", "tr").
    pub language: String,
    /// Current launcher version.
    pub launcher_version: String,
    /// Recently opened projects across suite applications, newest first.
    pub recent_projects: Vec<RecentProject>,
}

impl Default for SharedPreferences {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            launcher_version: env!("CARGO_PKG_VERSION").to_string(),
            recent_projects: Vec::new(),
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

    /// Records a project as recently opened, moving duplicates to the top and capping history length.
    pub fn record_recent_project(&mut self, app_id: AppId, path: PathBuf) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(ToString::to_string)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        self.recent_projects
            .retain(|p| !(p.app_id == app_id && p.path == path));
        self.recent_projects.insert(
            0,
            RecentProject {
                app_id,
                path,
                name,
                last_opened_unix: now,
            },
        );
        self.recent_projects.truncate(MAX_RECENT_PROJECTS);
    }

    /// Removes a specific project entry from the recent list.
    pub fn remove_recent_project(&mut self, app_id: AppId, path: &Path) {
        self.recent_projects
            .retain(|p| !(p.app_id == app_id && p.path == path));
    }

    /// Clears all recent project entries.
    pub fn clear_recent_projects(&mut self) {
        self.recent_projects.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_preferences_serialization() {
        let mut prefs = SharedPreferences {
            language: "tr".to_string(),
            launcher_version: "0.1.0".to_string(),
            recent_projects: Vec::new(),
        };
        prefs.record_recent_project(AppId::Trim, PathBuf::from("/tmp/demo.trim"));

        let ron_str = ron::to_string(&prefs).unwrap();
        assert!(ron_str.contains("language") && ron_str.contains("\"tr\""));
        assert!(ron_str.contains("demo.trim"));

        let deserialized: SharedPreferences = ron::from_str(&ron_str).unwrap();
        assert_eq!(prefs, deserialized);
    }

    #[test]
    fn test_shared_preferences_backward_compat_without_recent_projects() {
        let legacy_ron = "(language:\"tr\",launcher_version:\"1.2.0\")";
        let prefs: SharedPreferences = ron::from_str(legacy_ron).unwrap();
        assert_eq!(prefs.language, "tr");
        assert!(prefs.recent_projects.is_empty());
    }

    #[test]
    fn test_record_recent_project_dedup_and_remove() {
        let mut prefs = SharedPreferences::default();
        let p1 = PathBuf::from("/tmp/one.beam");
        let p2 = PathBuf::from("/tmp/two.stem");

        prefs.record_recent_project(AppId::Beam, p1.clone());
        prefs.record_recent_project(AppId::Stem, p2.clone());
        prefs.record_recent_project(AppId::Beam, p1.clone());

        assert_eq!(prefs.recent_projects.len(), 2);
        assert_eq!(prefs.recent_projects[0].app_id, AppId::Beam);
        assert_eq!(prefs.recent_projects[0].name, "one.beam");

        prefs.remove_recent_project(AppId::Beam, &p1);
        assert_eq!(prefs.recent_projects.len(), 1);

        prefs.clear_recent_projects();
        assert!(prefs.recent_projects.is_empty());
    }
}
