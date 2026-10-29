use directories::ProjectDirs;
use prism_core::catalog::AppEntry;
use prism_core::error::PrismError;
use prism_core::state::AppInstallState;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Installation directory could not be determined")]
    NoInstallDir,

    #[error("Application executable not found at {0}")]
    ExecutableNotFound(PathBuf),
}

impl From<InstallError> for PrismError {
    fn from(err: InstallError) -> Self {
        PrismError::Install(err.to_string())
    }
}

/// Manages installation directories and application binaries on the local filesystem.
pub struct InstallationManager {
    custom_root: Option<PathBuf>,
}

impl InstallationManager {
    pub fn new(custom_root: Option<PathBuf>) -> Self {
        Self { custom_root }
    }

    /// Returns the root directory where suite applications are installed.
    pub fn apps_root_dir(&self) -> Result<PathBuf, InstallError> {
        if let Some(ref path) = self.custom_root {
            return Ok(path.clone());
        }

        let proj =
            ProjectDirs::from("com", "prism", "prism-suite").ok_or(InstallError::NoInstallDir)?;
        Ok(proj.data_dir().join("apps"))
    }

    /// Returns the dedicated directory for an individual app.
    pub fn app_dir(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        Ok(self.apps_root_dir()?.join(app.id.as_str()))
    }

    /// Returns the binary path of an installed app.
    pub fn app_binary_path(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        let app_dir = self.app_dir(app)?;
        let binary_name = if cfg!(windows) {
            format!("{}.exe", app.primary_binary_name)
        } else {
            app.primary_binary_name.clone()
        };

        // Standard structure: apps/<app_id>/current/<binary>
        Ok(app_dir.join("current").join(binary_name))
    }

    /// Inspects the filesystem to detect the current installed state of an app.
    pub fn detect_state(&self, app: &AppEntry) -> AppInstallState {
        match self.app_binary_path(app) {
            Ok(bin_path) => {
                if bin_path.exists() {
                    // Check version file if present
                    let version_file = bin_path.parent().map(|p| p.join("version.txt"));
                    let version = version_file
                        .and_then(|vf| std::fs::read_to_string(vf).ok())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "0.1.0".to_string());

                    AppInstallState::Installed { version }
                } else {
                    AppInstallState::NotInstalled
                }
            }
            Err(_) => AppInstallState::NotInstalled,
        }
    }
}

impl Default for InstallationManager {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::catalog::AppCatalog;
    use std::path::Path;

    #[test]
    fn test_install_manager_paths() {
        let mgr = InstallationManager::new(Some(Path::new("/tmp/test-prism").to_path_buf()));
        let catalog = AppCatalog::load_embedded();
        let trim = catalog.get(prism_core::AppId::Trim).unwrap();

        let dir = mgr.app_dir(trim).unwrap();
        assert_eq!(dir, Path::new("/tmp/test-prism/trim"));

        let bin = mgr.app_binary_path(trim).unwrap();
        if cfg!(windows) {
            assert_eq!(bin, Path::new("/tmp/test-prism/trim/current/trim.exe"));
        } else {
            assert_eq!(bin, Path::new("/tmp/test-prism/trim/current/trim"));
        }
    }
}
