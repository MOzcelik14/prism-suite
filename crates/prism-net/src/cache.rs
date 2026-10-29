use crate::github::GitHubRelease;
use chrono::Utc;
use directories::ProjectDirs;
use prism_core::catalog::AppId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Disk cache structure for GitHub releases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCacheEntry {
    pub timestamp: i64,
    pub releases: Vec<GitHubRelease>,
}

pub struct ReleaseCache;

impl ReleaseCache {
    fn cache_file_path(app_id: AppId) -> Option<PathBuf> {
        ProjectDirs::from("com", "prism", "prism-suite").map(|proj| {
            proj.cache_dir()
                .join("releases")
                .join(format!("{}.ron", app_id.as_str()))
        })
    }

    /// Loads cached releases if they exist and are within TTL.
    pub fn get(app_id: AppId, ttl_seconds: i64) -> Option<Vec<GitHubRelease>> {
        let path = Self::cache_file_path(app_id)?;
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(&path).ok()?;
        let entry: ReleaseCacheEntry = ron::from_str(&content).ok()?;

        let now = Utc::now().timestamp();
        if now - entry.timestamp < ttl_seconds {
            Some(entry.releases)
        } else {
            None
        }
    }

    /// Stores releases to disk cache.
    pub fn set(app_id: AppId, releases: Vec<GitHubRelease>) -> Result<(), std::io::Error> {
        if let Some(path) = Self::cache_file_path(app_id) {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let entry = ReleaseCacheEntry {
                timestamp: Utc::now().timestamp(),
                releases,
            };
            let content = ron::to_string(&entry).map_err(std::io::Error::other)?;
            fs::write(path, content)?;
        }
        Ok(())
    }
}
