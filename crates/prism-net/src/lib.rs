pub mod cache;
pub mod github;

use ed25519_dalek::VerifyingKey;
use prism_core::catalog::{AppEntry, AppId};
use prism_core::error::PrismError;
use prism_core::manifest::{TargetArch, TargetOs};
use prism_core::state::{ReleaseChannel, ReleaseInfo};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use cache::ReleaseCache;
pub use github::{find_newest_compatible_release, parse_version, GitHubAsset, GitHubRelease};

/// Compile-time configurable Ed25519 public key for release signature verification.
/// In production, set `PRISM_PUBLIC_KEY=<hex-or-base64>` at build time.
pub const DEFAULT_ED25519_PUBLIC_KEY_HEX: &str = match option_env!("PRISM_PUBLIC_KEY") {
    Some(val) => val,
    None => "0000000000000000000000000000000000000000000000000000000000000000",
};

#[derive(Debug, Error)]
pub enum NetError {
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

    /// Fetches all GitHub releases for a given repository with caching.
    pub async fn fetch_releases(
        &self,
        app_id: AppId,
        repository_url: &str,
        force_refresh: bool,
    ) -> Result<Vec<GitHubRelease>, NetError> {
        // Check cache first (15-minute default TTL = 900 seconds)
        if !force_refresh {
            if let Some(cached) = ReleaseCache::get(app_id, 900) {
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
                // If offline or network error, fallback to existing cache regardless of TTL
                if let Some(cached) = ReleaseCache::get(app_id, i64::MAX) {
                    return Ok(cached);
                }
                return Err(NetError::Offline(e.to_string()));
            }
        };

        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            // Check for rate limit
            if let Some(cached) = ReleaseCache::get(app_id, i64::MAX) {
                return Ok(cached);
            }
            return Err(NetError::RateLimited);
        }

        if status == reqwest::StatusCode::NOT_FOUND {
            // Repository exists but has no releases or repo is 404
            return Ok(Vec::new());
        }

        let releases: Vec<GitHubRelease> = response.error_for_status()?.json().await?;
        let _ = ReleaseCache::set(app_id, releases.clone());

        Ok(releases)
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
