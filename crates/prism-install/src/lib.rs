pub mod archive;
pub mod desktop;
pub mod process;
pub mod self_update;

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use prism_core::catalog::AppEntry;
use prism_core::error::PrismError;
use prism_core::state::AppInstallState;
use thiserror::Error;

pub use archive::{extract_archive, ArchiveError};
pub use desktop::{install_desktop_entry, remove_desktop_entry, DesktopError};
pub use process::{is_binary_running, launch_detached};
pub use self_update::apply_self_update;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Archive error: {0}")]
    Archive(#[from] ArchiveError),

    #[error("Desktop integration error: {0}")]
    Desktop(#[from] DesktopError),

    #[error("Installation directory could not be determined")]
    NoInstallDir,

    #[error("Application executable not found at {0}")]
    ExecutableNotFound(PathBuf),

    #[error("Rollback failed: No previous version available to restore")]
    NoPreviousVersion,

    #[error("Application is currently running")]
    AppCurrentlyRunning,
}

impl From<InstallError> for PrismError {
    fn from(err: InstallError) -> Self {
        PrismError::Install(err.to_string())
    }
}

/// Manages installation directories, application binaries, and process execution on the local filesystem.
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

    /// Returns the versions directory for an app: `apps/<app_id>/versions/`
    pub fn versions_dir(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        Ok(self.app_dir(app)?.join("versions"))
    }

    /// Returns the specific version directory for an app: `apps/<app_id>/versions/<version>/`
    pub fn version_dir(&self, app: &AppEntry, version: &str) -> Result<PathBuf, InstallError> {
        Ok(self.versions_dir(app)?.join(version))
    }

    /// Returns the path to the `current` symlink/junction: `apps/<app_id>/current`
    pub fn current_link_path(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        Ok(self.app_dir(app)?.join("current"))
    }

    /// Returns the path to the `previous` symlink: `apps/<app_id>/previous`
    pub fn previous_link_path(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        Ok(self.app_dir(app)?.join("previous"))
    }

    /// Returns the binary path of an installed app: `apps/<app_id>/current/<binary_name>`
    pub fn app_binary_path(&self, app: &AppEntry) -> Result<PathBuf, InstallError> {
        let app_dir = self.app_dir(app)?;
        let binary_name = if cfg!(windows) {
            format!("{}.exe", app.primary_binary_name)
        } else {
            app.primary_binary_name.clone()
        };

        let direct_bin = app_dir.join("current").join(&binary_name);
        if direct_bin.exists() {
            return Ok(direct_bin);
        }

        // Check if inside bin/ subdirectory
        let sub_bin = app_dir.join("current").join("bin").join(&binary_name);
        if sub_bin.exists() {
            return Ok(sub_bin);
        }

        Ok(direct_bin)
    }

    /// Detects the previous installed version if a valid `previous` link exists.
    pub fn detect_previous_version(&self, app: &AppEntry) -> Option<String> {
        let prev_path = self.previous_link_path(app).ok()?;
        if prev_path.exists() {
            if let Ok(target) = fs::read_link(&prev_path) {
                if let Some(name) = target.file_name().and_then(|n| n.to_str()) {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Inspects the filesystem to detect the current installed state of an app.
    pub fn detect_state(&self, app: &AppEntry) -> AppInstallState {
        match self.current_link_path(app) {
            Ok(cur_link) => {
                if cur_link.exists() {
                    let version_file = cur_link.join("version.txt");
                    let version = if version_file.exists() {
                        fs::read_to_string(version_file)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|_| "0.1.0".to_string())
                    } else if let Ok(target) = fs::read_link(&cur_link) {
                        target
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("0.1.0")
                            .to_string()
                    } else {
                        "0.1.0".to_string()
                    };

                    let previous_version = self.detect_previous_version(app);

                    AppInstallState::Installed {
                        version,
                        previous_version,
                        latest_release: None,
                    }
                } else {
                    AppInstallState::NotInstalled {
                        latest_release: None,
                    }
                }
            }
            Err(_) => AppInstallState::NotInstalled {
                latest_release: None,
            },
        }
    }

    /// Extracts and installs an archive package atomically into `versions/<version>/`.
    pub fn install_package(
        &self,
        app: &AppEntry,
        version: &str,
        filename: &str,
        archive_bytes: &[u8],
        keep_count: usize,
    ) -> Result<(), InstallError> {
        let app_dir = self.app_dir(app)?;
        let staging_dir = app_dir.join("staging").join(version);
        let target_version_dir = self.version_dir(app, version)?;

        // 1. Clean previous staging if any
        if staging_dir.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
        }
        fs::create_dir_all(&staging_dir)?;

        // 2. Safe extraction into staging
        extract_archive(filename, archive_bytes, &staging_dir)?;

        // Write version.txt if not present
        let ver_file = staging_dir.join("version.txt");
        if !ver_file.exists() {
            let _ = fs::write(ver_file, version);
        }

        // 3. Move from staging into target version dir
        if target_version_dir.exists() {
            let _ = fs::remove_dir_all(&target_version_dir);
        }
        if let Some(parent) = target_version_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staging_dir, &target_version_dir)?;
        let _ = fs::remove_dir(app_dir.join("staging"));

        // 4. Update previous link if current link already exists
        let current_link = self.current_link_path(app)?;
        let previous_link = self.previous_link_path(app)?;

        if current_link.exists() {
            if let Ok(existing_target) = fs::read_link(&current_link) {
                if existing_target != target_version_dir {
                    let _ = fs::remove_file(&previous_link);
                    create_symlink(&existing_target, &previous_link)?;
                }
            }
        }

        // 5. Update current link atomically
        self.switch_current_link(app, &target_version_dir)?;

        // 6. Prune old versions exceeding keep_count
        self.clean_old_versions(app, keep_count)?;

        // 7. Install desktop integration (.desktop file and shortcut)
        if let Ok(bin_path) = self.app_binary_path(app) {
            let _ = install_desktop_entry(app, &bin_path);
        }

        Ok(())
    }

    /// Switches the active `current` link atomically to `target_version_dir`.
    fn switch_current_link(
        &self,
        app: &AppEntry,
        target_version_dir: &Path,
    ) -> Result<(), InstallError> {
        let current_link = self.current_link_path(app)?;
        let temp_link = self.app_dir(app)?.join("current.tmp");

        #[cfg(unix)]
        {
            let _ = fs::remove_file(&temp_link);
            std::os::unix::fs::symlink(target_version_dir, &temp_link)?;
            fs::rename(&temp_link, &current_link)?;
        }

        #[cfg(windows)]
        {
            if current_link.exists() {
                let _ = fs::remove_dir_all(&current_link);
            }
            let _ = std::os::windows::fs::symlink_dir(target_version_dir, &current_link);
        }

        Ok(())
    }

    /// Rolls back the active version to the version referenced by the `previous` link.
    pub fn rollback(&self, app: &AppEntry) -> Result<String, InstallError> {
        let prev_link = self.previous_link_path(app)?;
        if !prev_link.exists() {
            return Err(InstallError::NoPreviousVersion);
        }

        let target = fs::read_link(&prev_link)?;
        let restored_version = target
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(InstallError::NoPreviousVersion)?
            .to_string();

        self.switch_current_link(app, &target)?;
        let _ = fs::remove_file(&prev_link);

        // Update desktop integration to point to active executable
        if let Ok(bin_path) = self.app_binary_path(app) {
            let _ = install_desktop_entry(app, &bin_path);
        }

        Ok(restored_version)
    }

    /// Launches an installed application as a detached process.
    pub fn launch_app(&self, app: &AppEntry) -> Result<u32, InstallError> {
        let bin_path = self.app_binary_path(app)?;
        if !bin_path.exists() {
            return Err(InstallError::ExecutableNotFound(bin_path));
        }

        let working_dir = bin_path.parent().unwrap_or_else(|| Path::new("."));

        launch_detached(&bin_path, working_dir).map_err(InstallError::Io)
    }

    /// Checks if the application is currently running.
    pub fn is_running(&self, app: &AppEntry, known_pid: Option<u32>) -> bool {
        is_binary_running(&app.primary_binary_name, known_pid)
    }

    /// Cleans older versions, keeping the specified number of newest versions.
    pub fn clean_old_versions(
        &self,
        app: &AppEntry,
        keep_count: usize,
    ) -> Result<(), InstallError> {
        let versions_dir = self.versions_dir(app)?;
        if !versions_dir.exists() {
            return Ok(());
        }

        let mut version_dirs = Vec::new();
        for entry in fs::read_dir(&versions_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                version_dirs.push(entry.path());
            }
        }

        version_dirs.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
        version_dirs.reverse();

        let current_target = self
            .current_link_path(app)
            .ok()
            .and_then(|p| fs::read_link(p).ok());
        let previous_target = self
            .previous_link_path(app)
            .ok()
            .and_then(|p| fs::read_link(p).ok());

        let mut preserved = 0;
        for path in version_dirs {
            let is_active =
                current_target.as_ref() == Some(&path) || previous_target.as_ref() == Some(&path);

            if is_active || preserved < keep_count {
                preserved += 1;
            } else {
                let _ = fs::remove_dir_all(&path);
            }
        }

        Ok(())
    }

    /// Uninstalls an application from disk.
    pub fn uninstall(&self, app: &AppEntry, remove_user_data: bool) -> Result<(), InstallError> {
        // Remove desktop integration first
        let _ = remove_desktop_entry(app);

        let app_dir = self.app_dir(app)?;
        if app_dir.exists() {
            fs::remove_dir_all(&app_dir)?;
        }

        if remove_user_data {
            if let Some(proj) = ProjectDirs::from("com", "prism", app.id.as_str()) {
                let _ = fs::remove_dir_all(proj.config_dir());
                let _ = fs::remove_dir_all(proj.data_dir());
            }
        }

        Ok(())
    }
}

fn create_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link)
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
