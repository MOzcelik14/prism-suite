pub mod branding;
pub mod catalog;
pub mod config;
pub mod error;
pub mod manifest;
pub mod state;

pub use branding::{
    GITHUB_ORG_URL, LAUNCHER_DISPLAY_NAME, LAUNCHER_REPO_URL, SUITE_DISPLAY_NAME, SUITE_LICENSE,
    SUITE_VERSION,
};
pub use catalog::{AppCatalog, AppEntry, AppId};
pub use config::LauncherConfig;
pub use error::PrismError;
pub use manifest::{AppReleaseManifest, ReleaseAsset, TargetArch, TargetOs};
pub use state::{AppInstallState, ReleaseChannel, ReleaseInfo};
