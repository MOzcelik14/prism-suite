// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use prism_core::catalog::AppId;
use prism_core::manifest::{TargetArch, TargetOs};
use prism_core::state::{ReleaseChannel, ReleaseInfo};
use semver::Version;
use serde::{Deserialize, Serialize};

/// Represents an individual file asset attached to a GitHub release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

/// Represents a GitHub Release entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

/// Helper to parse semver from tag name or filename (e.g. "v1.2.3" -> "1.2.3").
pub fn parse_version(raw: &str) -> Option<Version> {
    let clean = raw.trim().strip_prefix('v').unwrap_or(raw.trim());
    Version::parse(clean).ok()
}

/// Inspects GitHub releases and returns the newest compatible release for an AppId.
pub fn find_newest_compatible_release(
    releases: &[GitHubRelease],
    app_id: AppId,
    os: TargetOs,
    arch: TargetArch,
    channel: ReleaseChannel,
) -> Option<ReleaseInfo> {
    find_newest_compatible_release_for_name(releases, app_id.as_str(), os, arch, channel)
}

/// Inspects GitHub releases and returns the newest compatible release for a binary name.
pub fn find_newest_compatible_release_for_name(
    releases: &[GitHubRelease],
    app_name: &str,
    os: TargetOs,
    arch: TargetArch,
    channel: ReleaseChannel,
) -> Option<ReleaseInfo> {
    let app_prefix = format!("{}-", app_name);
    let target_suffix = format!("-{}-{}.", os.as_str(), arch.as_str());
    let expected_ext = match os {
        TargetOs::Linux => ".tar.gz",
        TargetOs::Windows => ".zip",
        TargetOs::MacOs => ".tar.gz",
    };

    let mut compatible_releases = Vec::new();

    for release in releases {
        if release.draft {
            continue;
        }

        if channel == ReleaseChannel::Stable && release.prerelease {
            continue;
        }

        // Search for matching target package asset
        let target_asset = release.assets.iter().find(|asset| {
            asset.name.starts_with(&app_prefix)
                && asset.name.contains(&target_suffix)
                && asset.name.ends_with(expected_ext)
        });

        if let Some(asset) = target_asset {
            // Extract version from filename or tag
            // Filename format: <app_id>-<version>-<os>-<arch>.<ext>
            let version = extract_version_from_filename(&asset.name, app_name, &target_suffix)
                .or_else(|| parse_version(&release.tag_name));

            if let Some(ver) = version {
                // Find optional checksums and signature files in the same release
                let checksums_url = release
                    .assets
                    .iter()
                    .find(|a| a.name == "SHA256SUMS")
                    .map(|a| a.browser_download_url.clone());

                let signature_url = release
                    .assets
                    .iter()
                    .find(|a| a.name == "SHA256SUMS.sig")
                    .map(|a| a.browser_download_url.clone());

                let release_info = ReleaseInfo {
                    version: ver,
                    tag_name: release.tag_name.clone(),
                    release_name: release
                        .name
                        .clone()
                        .unwrap_or_else(|| release.tag_name.clone()),
                    release_notes: release.body.clone().unwrap_or_default(),
                    published_at: release.published_at.clone(),
                    is_prerelease: release.prerelease,
                    asset_filename: asset.name.clone(),
                    asset_download_url: asset.browser_download_url.clone(),
                    asset_size: asset.size,
                    checksums_url,
                    signature_url,
                };
                compatible_releases.push(release_info);
            }
        }
    }

    // Sort by semver descending
    compatible_releases.sort_by(|a, b| b.version.cmp(&a.version));
    compatible_releases.into_iter().next()
}

fn extract_version_from_filename(
    filename: &str,
    app_name: &str,
    target_suffix: &str,
) -> Option<Version> {
    let without_prefix = filename.strip_prefix(&format!("{}-", app_name))?;
    let (ver_str, _) = without_prefix.split_once(target_suffix)?;
    parse_version(ver_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_and_sort_releases() {
        let releases = vec![
            GitHubRelease {
                tag_name: "v0.1.0".to_string(),
                name: Some("triM 0.1.0".to_string()),
                body: Some("Initial release".to_string()),
                draft: false,
                prerelease: false,
                published_at: Some("2026-10-01T12:00:00Z".to_string()),
                assets: vec![GitHubAsset {
                    name: "trim-0.1.0-linux-x86_64.tar.gz".to_string(),
                    size: 1024,
                    browser_download_url: "https://example.com/trim-0.1.0-linux-x86_64.tar.gz"
                        .to_string(),
                }],
            },
            GitHubRelease {
                tag_name: "v0.2.0-beta.1".to_string(),
                name: Some("triM 0.2.0 Beta 1".to_string()),
                body: Some("Beta features".to_string()),
                draft: false,
                prerelease: true,
                published_at: Some("2026-10-15T12:00:00Z".to_string()),
                assets: vec![GitHubAsset {
                    name: "trim-0.2.0-beta.1-linux-x86_64.tar.gz".to_string(),
                    size: 2048,
                    browser_download_url:
                        "https://example.com/trim-0.2.0-beta.1-linux-x86_64.tar.gz".to_string(),
                }],
            },
            GitHubRelease {
                tag_name: "v0.2.0".to_string(),
                name: Some("triM 0.2.0".to_string()),
                body: Some("Stable 0.2.0 release".to_string()),
                draft: false,
                prerelease: false,
                published_at: Some("2026-10-20T12:00:00Z".to_string()),
                assets: vec![
                    GitHubAsset {
                        name: "trim-0.2.0-linux-x86_64.tar.gz".to_string(),
                        size: 2048,
                        browser_download_url: "https://example.com/trim-0.2.0-linux-x86_64.tar.gz"
                            .to_string(),
                    },
                    GitHubAsset {
                        name: "SHA256SUMS".to_string(),
                        size: 120,
                        browser_download_url: "https://example.com/SHA256SUMS".to_string(),
                    },
                    GitHubAsset {
                        name: "SHA256SUMS.sig".to_string(),
                        size: 96,
                        browser_download_url: "https://example.com/SHA256SUMS.sig".to_string(),
                    },
                ],
            },
        ];

        // Stable channel: picks 0.2.0
        let stable = find_newest_compatible_release(
            &releases,
            AppId::Trim,
            TargetOs::Linux,
            TargetArch::X86_64,
            ReleaseChannel::Stable,
        )
        .expect("should find 0.2.0");
        assert_eq!(stable.version, Version::parse("0.2.0").unwrap());
        assert!(stable.checksums_url.is_some());
        assert!(stable.signature_url.is_some());

        // Incompatible OS check
        let win = find_newest_compatible_release(
            &releases,
            AppId::Trim,
            TargetOs::Windows,
            TargetArch::X86_64,
            ReleaseChannel::Stable,
        );
        assert!(win.is_none());
    }

    #[test]
    fn test_launcher_release_discovery() {
        let releases = vec![GitHubRelease {
            tag_name: "v0.2.0".to_string(),
            name: Some("prisM. Suite 0.2.0".to_string()),
            body: Some("Exciting launcher updates".to_string()),
            draft: false,
            prerelease: false,
            published_at: Some("2026-10-25T10:00:00Z".to_string()),
            assets: vec![
                GitHubAsset {
                    name: "prism-0.2.0-linux-x86_64.tar.gz".to_string(),
                    size: 4096,
                    browser_download_url: "https://example.com/prism-0.2.0-linux-x86_64.tar.gz"
                        .to_string(),
                },
                GitHubAsset {
                    name: "SHA256SUMS".to_string(),
                    size: 80,
                    browser_download_url: "https://example.com/SHA256SUMS".to_string(),
                },
            ],
        }];

        let launcher_release = find_newest_compatible_release_for_name(
            &releases,
            "prism",
            TargetOs::Linux,
            TargetArch::X86_64,
            ReleaseChannel::Stable,
        )
        .expect("should match launcher release");

        assert_eq!(launcher_release.version, Version::parse("0.2.0").unwrap());
        assert_eq!(
            launcher_release.asset_filename,
            "prism-0.2.0-linux-x86_64.tar.gz"
        );
    }
}
