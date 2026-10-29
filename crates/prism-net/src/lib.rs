// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

pub mod cache;
pub mod github;
pub mod verify;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use ed25519_dalek::VerifyingKey;
use prism_core::catalog::{AppEntry, AppId};
use prism_core::error::PrismError;
use prism_core::manifest::{TargetArch, TargetOs};
use prism_core::state::{ReleaseChannel, ReleaseInfo};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use cache::ReleaseCache;
pub use github::{
    find_newest_compatible_release, find_newest_compatible_release_for_name, parse_version,
    GitHubAsset, GitHubRelease,
};
pub use verify::{parse_signature_bytes, verify_ed25519_signature, verify_sha256sums};

/// Compile-time configurable Ed25519 public key for release signature verification.
/// In production, set `PRISM_PUBLIC_KEY=<hex-or-base64>` at build time.
pub const DEFAULT_ED25519_PUBLIC_KEY_HEX: &str = match option_env!("PRISM_PUBLIC_KEY") {
    Some(val) => val,
    None => "0000000000000000000000000000000000000000000000000000000000000000",
};

#[derive(Debug, Error)]
pub enum NetError {
    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("GitHub API rate limit exceeded. Please try again later.")]
    RateLimited,

    #[error("Network connection offline or unreachable: {0}")]
    Offline(String),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Checksum mismatch: expected {expected}, calculated {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Cryptographic signature verification failed: {0}")]
    SignatureInvalid(String),

    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),
}

impl From<NetError> for PrismError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::Cancelled => PrismError::Network("Download cancelled".to_string()),
            NetError::RateLimited => {
                PrismError::Network("GitHub API rate limit exceeded".to_string())
            }
            NetError::Offline(msg) => PrismError::Network(format!("Offline: {}", msg)),
            NetError::Http(e) => PrismError::Network(e.to_string()),
            NetError::ChecksumMismatch { .. } | NetError::SignatureInvalid(_) => {
                PrismError::Security(err.to_string())
            }
            NetError::InvalidPublicKey(msg) => PrismError::Security(msg),
        }
    }
}

/// Helper function to parse hex string into bytes without external hex crate.
pub fn parse_hex(hex_str: &str) -> Result<Vec<u8>, NetError> {
    let clean = hex_str.trim();
    if !clean.len().is_multiple_of(2) {
        return Err(NetError::InvalidPublicKey(
            "Hex string length must be even".to_string(),
        ));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&clean[i..i + 2], 16)
                .map_err(|e| NetError::InvalidPublicKey(format!("Invalid hex character: {}", e)))
        })
        .collect()
}

/// Network client for fetching releases and update manifests.
#[derive(Clone)]
pub struct NetworkClient {
    client: reqwest::Client,
}

impl NetworkClient {
    pub fn new() -> Result<Self, NetError> {
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "prisM-Suite/",
                env!("CARGO_PKG_VERSION"),
                " (desktop)"
            ))
            .build()?;
        Ok(Self { client })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Fetches all GitHub releases for a given repository with caching using an arbitrary cache key.
    pub async fn fetch_repo_releases(
        &self,
        cache_key: &str,
        repository_url: &str,
        force_refresh: bool,
    ) -> Result<Vec<GitHubRelease>, NetError> {
        // Check cache first (15-minute default TTL = 900 seconds)
        if !force_refresh {
            if let Some(cached) = ReleaseCache::get_by_key(cache_key, 900) {
                return Ok(cached);
            }
        }

        let (owner, repo) = extract_owner_and_repo(repository_url).ok_or_else(|| {
            NetError::Offline(format!("Invalid repository URL: {}", repository_url))
        })?;

        let api_url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);

        let response = match self
            .client
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if let Some(cached) = ReleaseCache::get_by_key(cache_key, i64::MAX) {
                    return Ok(cached);
                }
                return Err(NetError::Offline(e.to_string()));
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            if let Some(cached) = ReleaseCache::get_by_key(cache_key, i64::MAX) {
                return Ok(cached);
            }
            return Err(NetError::RateLimited);
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }

        let releases: Vec<GitHubRelease> = response.error_for_status()?.json().await?;
        let _ = ReleaseCache::set_by_key(cache_key, releases.clone());

        Ok(releases)
    }

    /// Fetches all GitHub releases for a given app with caching.
    pub async fn fetch_releases(
        &self,
        app_id: AppId,
        repository_url: &str,
        force_refresh: bool,
    ) -> Result<Vec<GitHubRelease>, NetError> {
        self.fetch_repo_releases(app_id.as_str(), repository_url, force_refresh)
            .await
    }

    /// Discovers the newest compatible release for the launcher itself.
    pub async fn discover_launcher_release(
        &self,
        channel: ReleaseChannel,
        force_refresh: bool,
    ) -> Result<Option<ReleaseInfo>, NetError> {
        let releases = self
            .fetch_repo_releases(
                "prism-launcher",
                prism_core::branding::LAUNCHER_REPO_URL,
                force_refresh,
            )
            .await?;
        let os = TargetOs::current();
        let arch = TargetArch::current();
        Ok(find_newest_compatible_release_for_name(
            &releases, "prism", os, arch, channel,
        ))
    }

    /// Discovers the newest compatible release for an app given current OS and CPU arch.
    pub async fn discover_release(
        &self,
        app: &AppEntry,
        channel: ReleaseChannel,
        force_refresh: bool,
    ) -> Result<Option<ReleaseInfo>, NetError> {
        let releases = self
            .fetch_releases(app.id, &app.repository, force_refresh)
            .await?;
        let os = TargetOs::current();
        let arch = TargetArch::current();
        Ok(find_newest_compatible_release(
            &releases, app.id, os, arch, channel,
        ))
    }

    /// Downloads a resource over HTTP with chunk streaming, progress callbacks, and cancellation support.
    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        cancel_flag: Option<Arc<AtomicBool>>,
        mut on_progress: F,
    ) -> Result<Vec<u8>, NetError>
    where
        F: FnMut(u64, Option<u64>, f32) + Send + 'static,
    {
        if let Some(ref cancel) = cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                return Err(NetError::Cancelled);
            }
        }

        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| NetError::Offline(e.to_string()))?
            .error_for_status()?;

        let total_size = response.content_length();
        let mut downloaded: u64 = 0;
        let mut buffer = Vec::with_capacity(total_size.unwrap_or(1024 * 1024) as usize);

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| NetError::Offline(e.to_string()))?
        {
            if let Some(ref cancel) = cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    return Err(NetError::Cancelled);
                }
            }

            buffer.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;

            let progress = if let Some(total) = total_size {
                if total > 0 {
                    (downloaded as f32 / total as f32).min(1.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };

            on_progress(downloaded, total_size, progress);
        }

        Ok(buffer)
    }

    /// Downloads text content (e.g. SHA256SUMS or SHA256SUMS.sig).
    pub async fn fetch_text(&self, url: &str) -> Result<String, NetError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| NetError::Offline(e.to_string()))?
            .error_for_status()?;
        resp.text()
            .await
            .map_err(|e| NetError::Offline(e.to_string()))
    }

    /// Verifies that data matches an expected SHA-256 hash (hex string).
    pub fn verify_sha256(data: &[u8], expected_hex: &str) -> Result<(), NetError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let actual = format!("{:x}", hasher.finalize());
        if actual.eq_ignore_ascii_case(expected_hex.trim()) {
            Ok(())
        } else {
            Err(NetError::ChecksumMismatch {
                expected: expected_hex.to_string(),
                actual,
            })
        }
    }

    /// Parses the public key configured for release verification.
    pub fn public_key() -> Result<VerifyingKey, NetError> {
        let bytes = parse_hex(DEFAULT_ED25519_PUBLIC_KEY_HEX)?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| NetError::InvalidPublicKey("Public key must be 32 bytes".to_string()))?;

        VerifyingKey::from_bytes(&key_bytes).map_err(|e| NetError::InvalidPublicKey(e.to_string()))
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new().expect("failed to create default NetworkClient")
    }
}

/// Parses "https://github.com/owner/repo" into ("owner", "repo").
fn extract_owner_and_repo(url: &str) -> Option<(&str, &str)> {
    let stripped = url
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")?;
    let mut parts = stripped.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    Some((owner, repo))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_owner_and_repo() {
        assert_eq!(
            extract_owner_and_repo("https://github.com/MOzcelik14/triM"),
            Some(("MOzcelik14", "triM"))
        );
        assert_eq!(
            extract_owner_and_repo("https://github.com/MOzcelik14/steM/"),
            Some(("MOzcelik14", "steM"))
        );
        assert_eq!(extract_owner_and_repo("invalid-url"), None);
    }

    #[test]
    fn test_sha256_verification() {
        let data = b"hello prism suite";
        let expected = "1b8fdfe9172d93c30f2012f7bb6827d8113d75cf88ec5c2b41cac5b4b087b6af";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let calculated = format!("{:x}", hasher.finalize());
        assert_eq!(calculated, expected);

        assert!(NetworkClient::verify_sha256(data, expected).is_ok());
        assert!(NetworkClient::verify_sha256(data, "invalidchecksum").is_err());
    }

    #[test]
    fn test_parse_hex() {
        let bytes = parse_hex("deadbeef").unwrap();
        assert_eq!(bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
