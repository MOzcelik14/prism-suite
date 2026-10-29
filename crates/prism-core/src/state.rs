use semver::Version;
use serde::{Deserialize, Serialize};

/// Update release channel preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    #[default]
    Stable,
    Beta,
}

impl ReleaseChannel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReleaseChannel::Stable => "stable",
            ReleaseChannel::Beta => "beta",
        }
    }
}

/// Metadata and asset information for a remote application release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: Version,
    pub tag_name: String,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub is_prerelease: bool,
    pub asset_filename: String,
    pub asset_download_url: String,
    pub asset_size: u64,
    pub checksums_url: Option<String>,
    pub signature_url: Option<String>,
}

/// Installation and operational state of an individual suite app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppInstallState {
    NotInstalled {
        latest_release: Option<ReleaseInfo>,
    },
    NoReleaseAvailable,
    Installed {
        version: String,
        latest_release: Option<ReleaseInfo>,
    },
    UpdateAvailable {
        current_version: String,
        latest_release: ReleaseInfo,
    },
    Downloading {
        progress: f32,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
    },
    Verifying,
    Installing,
    Running,
    Error {
        message: String,
    },
}

impl AppInstallState {
    pub fn is_installed(&self) -> bool {
        matches!(
            self,
            AppInstallState::Installed { .. }
                | AppInstallState::UpdateAvailable { .. }
                | AppInstallState::Running
        )
    }

    pub fn is_busy(&self) -> bool {
        matches!(
            self,
            AppInstallState::Downloading { .. }
                | AppInstallState::Verifying
                | AppInstallState::Installing
                | AppInstallState::Running
        )
    }

    pub fn installed_version(&self) -> Option<&str> {
        match self {
            AppInstallState::Installed { version, .. } => Some(version.as_str()),
            AppInstallState::UpdateAvailable {
                current_version, ..
            } => Some(current_version.as_str()),
            _ => None,
        }
    }

    pub fn latest_release(&self) -> Option<&ReleaseInfo> {
        match self {
            AppInstallState::NotInstalled { latest_release } => latest_release.as_ref(),
            AppInstallState::Installed { latest_release, .. } => latest_release.as_ref(),
            AppInstallState::UpdateAvailable { latest_release, .. } => Some(latest_release),
            _ => None,
        }
    }
}
