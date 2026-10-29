use serde::{Deserialize, Serialize};

/// Installation and operational state of an individual suite app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppInstallState {
    NotInstalled,
    Installed {
        version: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
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
            AppInstallState::Installed { version } => Some(version.as_str()),
            AppInstallState::UpdateAvailable {
                current_version, ..
            } => Some(current_version.as_str()),
            _ => None,
        }
    }
}
