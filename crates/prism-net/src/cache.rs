// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

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
    fn cache_file_path_for_key(key: &str) -> Option<PathBuf> {
        ProjectDirs::from("com", "prism", "prism-suite").map(|proj| {
            proj.cache_dir()
                .join("releases")
                .join(format!("{}.ron", key))
        })
    }

    /// Loads cached releases by arbitrary key if they exist and are within TTL.
    pub fn get_by_key(key: &str, ttl_seconds: i64) -> Option<Vec<GitHubRelease>> {
        let path = Self::cache_file_path_for_key(key)?;
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

    /// Stores releases to disk cache by arbitrary key.
    pub fn set_by_key(key: &str, releases: Vec<GitHubRelease>) -> Result<(), std::io::Error> {
        if let Some(path) = Self::cache_file_path_for_key(key) {
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

    /// Loads cached releases for an app ID if they exist and are within TTL.
    pub fn get(app_id: AppId, ttl_seconds: i64) -> Option<Vec<GitHubRelease>> {
        Self::get_by_key(app_id.as_str(), ttl_seconds)
    }

    /// Stores releases to disk cache for an app ID.
    pub fn set(app_id: AppId, releases: Vec<GitHubRelease>) -> Result<(), std::io::Error> {
        Self::set_by_key(app_id.as_str(), releases)
    }
}
