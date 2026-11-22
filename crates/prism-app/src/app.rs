// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use egui_commonmark::CommonMarkCache;
use prism_core::branding::{LAUNCHER_DISPLAY_NAME, SUITE_VERSION};
use prism_core::catalog::{AppCatalog, AppId};
use prism_core::config::LauncherConfig;
use prism_core::shared::SharedPreferences;
use prism_core::state::{AppInstallState, ReleaseInfo};
use prism_i18n::{I18n, Language};
use prism_install::InstallationManager;
use prism_net::{verify_ed25519_signature, verify_sha256sums, NetError, NetworkClient};
use semver::Version;

/// Navigation tabs in the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Apps,
    RecentProjects,
    WhatsNew,
}

/// Filter categories for the app grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppFilter {
    #[default]
    All,
    Installed,
    Updates,
}

/// Operational state of the launcher self-updater.
#[derive(Debug, Clone, PartialEq)]
pub enum LauncherSelfUpdateState {
    Idle,
    UpdateAvailable(Box<ReleaseInfo>),
    Updating { progress: f32 },
    Updated { version: String },
    Error(String),
}

/// Events sent from background worker tasks to the UI thread.
pub enum AppEvent {
    ReleaseDiscovered {
        app_id: AppId,
        release: Option<ReleaseInfo>,
    },
    DiscoveryError {
        _app_id: AppId,
        error: NetError,
    },
    DownloadProgress {
        app_id: AppId,
        progress: f32,
        bytes_downloaded: u64,
        total_bytes: Option<u64>,
    },
    StateChange {
        app_id: AppId,
        new_state: AppInstallState,
    },
    OperationSuccess {
        _app_id: AppId,
        message: String,
    },
    OperationFailed {
        app_id: AppId,
        error: String,
    },
    LauncherReleaseDiscovered {
        release: Option<ReleaseInfo>,
    },
    LauncherUpdateProgress {
        progress: f32,
    },
    LauncherUpdated {
        version: String,
        target_exe: std::path::PathBuf,
    },
    LauncherUpdateFailed {
        error: String,
    },
}

#[derive(Clone)]
struct EventSender {
    tx: Sender<AppEvent>,
    ctx: egui::Context,
}

impl EventSender {
    fn send(&self, event: AppEvent) -> bool {
        let ok = self.tx.send(event).is_ok();
        self.ctx.request_repaint();
        ok
    }
}

/// Main application structure for prisM. Suite launcher.
pub struct PrismApp {
    pub(crate) catalog: AppCatalog,
    pub(crate) i18n: I18n,
    pub(crate) config: LauncherConfig,
    pub(crate) install_mgr: InstallationManager,
    net_client: NetworkClient,
    tokio_runtime: tokio::runtime::Runtime,
    event_tx: EventSender,
    event_rx: Receiver<AppEvent>,
    pub(crate) markdown_cache: CommonMarkCache,
    pub(crate) states: HashMap<AppId, AppInstallState>,
    running_pids: HashMap<AppId, u32>,
    cancel_tokens: HashMap<AppId, Arc<AtomicBool>>,
    pub(crate) selected_detail_app: Option<AppId>,
    pub(crate) uninstall_target_app: Option<AppId>,
    pub(crate) uninstall_remove_data: bool,
    pub(crate) show_settings: bool,
    pub(crate) show_about: bool,
    pub(crate) is_checking_releases: bool,
    last_process_poll: std::time::Instant,
    pub(crate) status_message: Option<(String, std::time::Instant)>,
    pub(crate) active_tab: ActiveTab,
    pub(crate) app_filter: AppFilter,
    pub(crate) launcher_update_state: LauncherSelfUpdateState,
    pub(crate) dismissed_launcher_update: bool,
    pub(crate) discovered_releases: Vec<(String, ReleaseInfo)>,
    pub(crate) custom_dir_input: String,
    pub(crate) shared_prefs: SharedPreferences,
    logo_textures: HashMap<u32, egui::TextureHandle>,
    pub(crate) app_textures: HashMap<AppId, egui::TextureHandle>,
}

impl PrismApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let catalog = AppCatalog::load_embedded();
        let config = LauncherConfig::load();
        let initial_lang = config.language.as_deref().map(Language::from_code);
        let i18n = I18n::new(initial_lang);
        let install_mgr = InstallationManager::new(config.custom_install_dir.clone());
        let net_client = NetworkClient::new().unwrap_or_default();
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("failed to start tokio runtime");

        let (tx, event_rx) = crossbeam_channel::unbounded();
        let event_tx = EventSender {
            tx,
            ctx: cc.egui_ctx.clone(),
        };

        // Preserve recent_projects while syncing language & launcher_version for child apps
        let mut shared_prefs = SharedPreferences::load();
        shared_prefs.language = i18n.language().as_code().to_string();
        shared_prefs.launcher_version = SUITE_VERSION.to_string();
        let _ = shared_prefs.save();

        // Initial states: detect installed state and running status from filesystem
        let mut states = HashMap::new();
        let running_pids = HashMap::new();

        for app in catalog.all() {
            let is_running = install_mgr.is_running(app, None);
            if is_running {
                states.insert(app.id, AppInstallState::Running);
            } else {
                let detected = install_mgr.detect_state(app);
                states.insert(app.id, detected);
            }
        }

        let custom_dir_input = config
            .custom_install_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut app = Self {
            catalog,
            i18n,
            config,
            install_mgr,
            net_client,
            tokio_runtime,
            event_tx,
            event_rx,
            markdown_cache: CommonMarkCache::default(),
            states,
            running_pids,
            cancel_tokens: HashMap::new(),
            selected_detail_app: None,
            uninstall_target_app: None,
            uninstall_remove_data: false,
            show_settings: false,
            show_about: false,
            is_checking_releases: false,
            last_process_poll: std::time::Instant::now(),
            status_message: None,
            active_tab: ActiveTab::default(),
            app_filter: AppFilter::default(),
            launcher_update_state: LauncherSelfUpdateState::Idle,
            dismissed_launcher_update: false,
            discovered_releases: Vec::new(),
            custom_dir_input,
            shared_prefs,
            logo_textures: HashMap::new(),
            app_textures: HashMap::new(),
        };

        if app.config.should_check_on_startup() {
            app.check_for_updates(false);
        }

        app
    }

    /// Spawns background worker tasks to fetch remote releases without blocking the UI.
    pub fn check_for_updates(&mut self, force_refresh: bool) {
        self.is_checking_releases = true;
        let apps = self.catalog.all().to_vec();
        let channel = self.config.release_channel;

        // Record check timestamp
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.config.last_update_check = Some(now);
        let _ = self.config.save();

        for app in apps {
            let client = self.net_client.clone();
            let tx = self.event_tx.clone();

            self.tokio_runtime.spawn(async move {
                match client.discover_release(&app, channel, force_refresh).await {
                    Ok(release) => {
                        let _ = tx.send(AppEvent::ReleaseDiscovered {
                            app_id: app.id,
                            release,
                        });
                    }
                    Err(error) => {
                        let _ = tx.send(AppEvent::DiscoveryError {
                            _app_id: app.id,
                            error,
                        });
                    }
                }
            });
        }

        // Check for launcher self-update
        let client = self.net_client.clone();
        let tx = self.event_tx.clone();
        self.tokio_runtime.spawn(async move {
            match client
                .discover_launcher_release(channel, force_refresh)
                .await
            {
                Ok(release) => {
                    let _ = tx.send(AppEvent::LauncherReleaseDiscovered { release });
                }
                Err(error) => {
                    log::warn!("Launcher update discovery failed: {}", error);
                }
            }
        });
    }

    /// Triggers launcher self-update download, verification, and executable replacement.
    pub fn trigger_launcher_self_update(&mut self, release: ReleaseInfo) {
        self.launcher_update_state = LauncherSelfUpdateState::Updating { progress: 0.0 };
        self.set_temporary_status(self.i18n.text("status-launcher-updating"));
        let client = self.net_client.clone();
        let tx = self.event_tx.clone();

        self.tokio_runtime.spawn(async move {
            let tx_prog = tx.clone();
            let on_progress = move |_bytes, _total, frac| {
                let _ = tx_prog.send(AppEvent::LauncherUpdateProgress { progress: frac });
            };

            let archive_bytes = match client
                .download_with_progress(&release.asset_download_url, None, on_progress)
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(AppEvent::LauncherUpdateFailed {
                        error: format!("Failed to download launcher update: {}", e),
                    });
                    return;
                }
            };

            // Optional checksums check
            if let Some(ref checksums_url) = release.checksums_url {
                if let Ok(sums_bytes) = client
                    .download_with_progress(checksums_url, None, |_, _, _| {})
                    .await
                {
                    if let Ok(sums_str) = std::str::from_utf8(&sums_bytes) {
                        let _ = prism_net::verify_sha256sums(
                            &release.asset_filename,
                            &archive_bytes,
                            sums_str,
                        );
                    }
                }
            }

            let current_exe = match std::env::current_exe() {
                Ok(p) => p,
                Err(e) => {
                    let _ = tx.send(AppEvent::LauncherUpdateFailed {
                        error: format!("Could not locate current executable: {}", e),
                    });
                    return;
                }
            };

            match prism_install::apply_self_update(
                &release.asset_filename,
                &archive_bytes,
                &current_exe,
            ) {
                Ok(target_exe) => {
                    let _ = tx.send(AppEvent::LauncherUpdated {
                        version: release.version.to_string(),
                        target_exe,
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::LauncherUpdateFailed {
                        error: format!("Failed to apply launcher self-update: {}", e),
                    });
                }
            }
        });
    }

    /// Launches an application as a detached process.
    pub fn launch_application(&mut self, app_id: AppId) {
        if let Some(app) = self.catalog.get(app_id) {
            match self.install_mgr.launch_app(app) {
                Ok(pid) => {
                    self.running_pids.insert(app_id, pid);
                    self.states.insert(app_id, AppInstallState::Running);
                    self.set_temporary_status(self.i18n.text("msg-launched-success"));
                }
                Err(e) => {
                    self.set_temporary_status(format!("Launch failed: {}", e));
                }
            }
        }
    }

    /// Launches an installed application with a project file argument and updates recent history.
    pub fn launch_application_with_file(&mut self, app_id: AppId, file_path: &std::path::Path) {
        let Some(app) = self.catalog.get(app_id).cloned() else {
            return;
        };
        let arg = file_path.to_string_lossy().into_owned();
        match self.install_mgr.launch_app_with_args(&app, &[&arg]) {
            Ok(pid) => {
                self.running_pids.insert(app_id, pid);
                self.states.insert(app_id, AppInstallState::Running);
                self.shared_prefs
                    .record_recent_project(app_id, file_path.to_path_buf());
                let _ = self.shared_prefs.save();
                self.set_temporary_status(self.i18n.text("msg-launched-success"));
            }
            Err(e) => {
                self.set_temporary_status(format!("Launch failed: {}", e));
            }
        }
    }

    /// Removes a single project from the recent projects list and persists shared preferences.
    pub fn remove_recent_project(&mut self, app_id: AppId, path: &std::path::Path) {
        self.shared_prefs.remove_recent_project(app_id, path);
        let _ = self.shared_prefs.save();
    }

    /// Clears all recent projects and persists shared preferences.
    pub fn clear_recent_projects(&mut self) {
        self.shared_prefs.clear_recent_projects();
        let _ = self.shared_prefs.save();
    }

    /// Triggers the full verified download, verify, extract, and atomic install flow.
    pub fn start_install_pipeline(&mut self, app_id: AppId, release: ReleaseInfo) {
        let app = match self.catalog.get(app_id) {
            Some(a) => a.clone(),
            None => return,
        };

        // Safety guard: prevent install/update if app is running
        if self
            .install_mgr
            .is_running(&app, self.running_pids.get(&app_id).copied())
        {
            self.set_temporary_status(self.i18n.text("error-app-running"));
            return;
        }

        let cancel_token = Arc::new(AtomicBool::new(false));
        self.cancel_tokens.insert(app_id, cancel_token.clone());

        self.states.insert(
            app_id,
            AppInstallState::Downloading {
                progress: 0.0,
                bytes_downloaded: 0,
                total_bytes: Some(release.asset_size),
            },
        );

        let client = self.net_client.clone();
        let install_mgr = InstallationManager::new(self.config.custom_install_dir.clone());
        let keep_count = self.config.keep_versions_count;
        let tx = self.event_tx.clone();

        self.tokio_runtime.spawn(async move {
            let app_id = app.id;
            let asset_url = release.asset_download_url.clone();
            let asset_filename = release.asset_filename.clone();
            let target_version = release.version.to_string();

            // 1. Download asset stream with progress
            let tx_prog = tx.clone();
            let download_res = client
                .download_with_progress(
                    &asset_url,
                    Some(cancel_token.clone()),
                    move |bytes, total, prog| {
                        let _ = tx_prog.send(AppEvent::DownloadProgress {
                            app_id,
                            progress: prog,
                            bytes_downloaded: bytes,
                            total_bytes: total,
                        });
                    },
                )
                .await;

            let archive_bytes = match download_res {
                Ok(bytes) => bytes,
                Err(NetError::Cancelled) => {
                    let _ = tx.send(AppEvent::OperationFailed {
                        app_id,
                        error: "Download cancelled by user".to_string(),
                    });
                    return;
                }
                Err(e) => {
                    if let Some(local_path) = find_local_project_binary(app_id) {
                        if install_mgr
                            .install_local_binary(&app, &target_version, &local_path)
                            .is_ok()
                        {
                            let _ = tx.send(AppEvent::StateChange {
                                app_id,
                                new_state: AppInstallState::Installed {
                                    version: target_version,
                                    previous_version: None,
                                    latest_release: Some(release.clone()),
                                },
                            });
                            return;
                        }
                    }

                    let _ = tx.send(AppEvent::OperationFailed {
                        app_id,
                        error: format!("Download failed: {}", e),
                    });
                    return;
                }
            };

            // 2. Verifying
            let _ = tx.send(AppEvent::StateChange {
                app_id,
                new_state: AppInstallState::Verifying,
            });

            if let Some(ref checksums_url) = release.checksums_url {
                if let Ok(manifest_content) = client.fetch_text(checksums_url).await {
                    if let Err(e) =
                        verify_sha256sums(&asset_filename, &archive_bytes, &manifest_content)
                    {
                        let _ = tx.send(AppEvent::OperationFailed {
                            app_id,
                            error: format!("Checksum verification failed: {}", e),
                        });
                        return;
                    }

                    if let Some(ref sig_url) = release.signature_url {
                        if let Ok(sig_content) = client.fetch_text(sig_url).await {
                            if let Ok(pubkey) = NetworkClient::public_key() {
                                if let Err(e) = verify_ed25519_signature(
                                    manifest_content.as_bytes(),
                                    &sig_content,
                                    &pubkey,
                                ) {
                                    let _ = tx.send(AppEvent::OperationFailed {
                                        app_id,
                                        error: format!("Signature verification failed: {}", e),
                                    });
                                    return;
                                }
                            }
                        }
                    }
                }
            }

            // 3. Installing
            let _ = tx.send(AppEvent::StateChange {
                app_id,
                new_state: AppInstallState::Installing,
            });

            match install_mgr.install_package(
                &app,
                &target_version,
                &asset_filename,
                &archive_bytes,
                keep_count,
            ) {
                Ok(()) => {
                    let prev = install_mgr.detect_previous_version(&app);
                    let _ = tx.send(AppEvent::StateChange {
                        app_id,
                        new_state: AppInstallState::Installed {
                            version: target_version,
                            previous_version: prev,
                            latest_release: Some(release),
                        },
                    });
                    let _ = tx.send(AppEvent::OperationSuccess {
                        _app_id: app_id,
                        message: "Application installed successfully.".to_string(),
                    });
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::OperationFailed {
                        app_id,
                        error: format!("Installation error: {}", e),
                    });
                }
            }
        });
    }

    /// Cancels an in-progress download for an application.
    pub fn cancel_download(&mut self, app_id: AppId) {
        if let Some(token) = self.cancel_tokens.remove(&app_id) {
            token.store(true, Ordering::Relaxed);
        }
        let detected = self
            .install_mgr
            .detect_state(self.catalog.get(app_id).unwrap());
        self.states.insert(app_id, detected);
    }

    /// Triggers a rollback to the previous version.
    pub fn trigger_rollback(&mut self, app_id: AppId) {
        if let Some(app) = self.catalog.get(app_id) {
            if self
                .install_mgr
                .is_running(app, self.running_pids.get(&app_id).copied())
            {
                self.set_temporary_status(self.i18n.text("error-app-running"));
                return;
            }

            match self.install_mgr.rollback(app) {
                Ok(restored) => {
                    let prev = self.install_mgr.detect_previous_version(app);
                    self.states.insert(
                        app_id,
                        AppInstallState::Installed {
                            version: restored,
                            previous_version: prev,
                            latest_release: None,
                        },
                    );
                    self.set_temporary_status(self.i18n.text("msg-rollback-success"));
                }
                Err(e) => {
                    self.set_temporary_status(format!("Rollback failed: {}", e));
                }
            }
        }
    }

    /// Executes uninstallation for the selected target app with safety check.
    pub fn execute_uninstall(&mut self, app_id: AppId, remove_user_data: bool) {
        if let Some(app) = self.catalog.get(app_id) {
            if self
                .install_mgr
                .is_running(app, self.running_pids.get(&app_id).copied())
            {
                self.set_temporary_status(self.i18n.text("error-app-running"));
                self.uninstall_target_app = None;
                return;
            }

            match self.install_mgr.uninstall(app, remove_user_data) {
                Ok(()) => {
                    self.states.insert(
                        app_id,
                        AppInstallState::NotInstalled {
                            latest_release: None,
                        },
                    );
                    self.set_temporary_status(self.i18n.text("msg-uninstall-success"));
                }
                Err(e) => {
                    self.set_temporary_status(format!("Uninstall failed: {}", e));
                }
            }
        }
        self.uninstall_target_app = None;
    }

    pub(crate) fn set_temporary_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    /// Returns a texture of the SVG logo rasterized for `points` logical size at the
    /// current display scale, so it stays sharp at any size or DPI.
    pub(crate) fn suite_logo_texture(
        &mut self,
        ctx: &egui::Context,
        points: f32,
    ) -> Option<egui::TextureId> {
        let px = (points * ctx.pixels_per_point()).round().max(8.0) as u32;
        if let std::collections::hash_map::Entry::Vacant(entry) = self.logo_textures.entry(px) {
            let image = crate::logo::render_logo_image(px)?;
            entry.insert(ctx.load_texture(
                format!("prism_logo_{px}"),
                image,
                egui::TextureOptions::LINEAR,
            ));
        }
        self.logo_textures.get(&px).map(|t| t.id())
    }

    fn poll_process_status(&mut self) {
        // Poll every 1.5 seconds to detect running/closed applications
        if self.last_process_poll.elapsed().as_millis() < 1500 {
            return;
        }
        self.last_process_poll = std::time::Instant::now();

        let apps = self.catalog.all().to_vec();
        for app in apps {
            let pid = self.running_pids.get(&app.id).copied();
            let is_running = self.install_mgr.is_running(&app, pid);

            let current =
                self.states
                    .get(&app.id)
                    .cloned()
                    .unwrap_or(AppInstallState::NotInstalled {
                        latest_release: None,
                    });

            if is_running {
                if current != AppInstallState::Running {
                    self.states.insert(app.id, AppInstallState::Running);
                }
            } else if current == AppInstallState::Running {
                self.running_pids.remove(&app.id);
                let detected = self.install_mgr.detect_state(&app);
                self.states.insert(app.id, detected);
            }
        }
    }

    fn handle_incoming_events(&mut self) {
        let mut completed_count = 0;
        while let Ok(event) = self.event_rx.try_recv() {
            completed_count += 1;
            match event {
                AppEvent::ReleaseDiscovered { app_id, release } => {
                    let current = self.states.get(&app_id).cloned().unwrap_or(
                        AppInstallState::NotInstalled {
                            latest_release: None,
                        },
                    );

                    let new_state = match current {
                        AppInstallState::Installed {
                            version,
                            previous_version,
                            ..
                        } => {
                            if let Some(ref rel) = release {
                                let inst_ver = Version::parse(&version).ok();
                                if let Some(iv) = inst_ver {
                                    if rel.version > iv {
                                        AppInstallState::UpdateAvailable {
                                            current_version: version,
                                            previous_version,
                                            latest_release: rel.clone(),
                                        }
                                    } else {
                                        AppInstallState::Installed {
                                            version,
                                            previous_version,
                                            latest_release: Some(rel.clone()),
                                        }
                                    }
                                } else {
                                    AppInstallState::Installed {
                                        version,
                                        previous_version,
                                        latest_release: Some(rel.clone()),
                                    }
                                }
                            } else {
                                AppInstallState::Installed {
                                    version,
                                    previous_version,
                                    latest_release: None,
                                }
                            }
                        }
                        AppInstallState::UpdateAvailable {
                            current_version,
                            previous_version,
                            ..
                        } => {
                            if let Some(ref rel) = release {
                                AppInstallState::UpdateAvailable {
                                    current_version,
                                    previous_version,
                                    latest_release: rel.clone(),
                                }
                            } else {
                                AppInstallState::Installed {
                                    version: current_version,
                                    previous_version,
                                    latest_release: None,
                                }
                            }
                        }
                        AppInstallState::Downloading { .. }
                        | AppInstallState::Verifying
                        | AppInstallState::Installing
                        | AppInstallState::Running => current,
                        _ => {
                            if let Some(ref rel) = release {
                                AppInstallState::NotInstalled {
                                    latest_release: Some(rel.clone()),
                                }
                            } else {
                                AppInstallState::NoReleaseAvailable
                            }
                        }
                    };

                    if let Some(ref rel) = release {
                        if let Some(app) = self.catalog.get(app_id) {
                            self.discovered_releases
                                .retain(|(name, _)| name != &app.display_name);
                            self.discovered_releases
                                .push((app.display_name.clone(), rel.clone()));
                            self.discovered_releases
                                .sort_by(|a, b| b.1.version.cmp(&a.1.version));
                        }
                    }

                    self.states.insert(app_id, new_state);
                }
                AppEvent::LauncherReleaseDiscovered { release } => {
                    if let Some(rel) = release {
                        self.discovered_releases
                            .retain(|(name, _)| name != LAUNCHER_DISPLAY_NAME);
                        self.discovered_releases
                            .push((LAUNCHER_DISPLAY_NAME.to_string(), rel.clone()));
                        self.discovered_releases
                            .sort_by(|a, b| b.1.version.cmp(&a.1.version));

                        let current_ver =
                            Version::parse(SUITE_VERSION).unwrap_or_else(|_| Version::new(0, 1, 0));
                        if rel.version > current_ver {
                            if self.config.auto_update {
                                self.trigger_launcher_self_update(rel);
                            } else {
                                self.launcher_update_state =
                                    LauncherSelfUpdateState::UpdateAvailable(Box::new(rel));
                                self.dismissed_launcher_update = false;
                            }
                        }
                    }
                }
                AppEvent::LauncherUpdateProgress { progress } => {
                    self.launcher_update_state = LauncherSelfUpdateState::Updating { progress };
                }
                AppEvent::LauncherUpdated {
                    version,
                    target_exe,
                } => {
                    self.launcher_update_state = LauncherSelfUpdateState::Updated {
                        version: version.clone(),
                    };
                    self.set_temporary_status(self.i18n.text_with_arg(
                        "msg-launcher-updated",
                        "version",
                        &version,
                    ));
                    #[cfg(not(windows))]
                    {
                        let work_dir = target_exe
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new("."));
                        let _ = prism_install::launch_detached(&target_exe, work_dir);
                    }
                    // On Windows, the detached cmd.exe helper waits for this PID to exit,
                    // finalizes replacement/cleanup, and relaunches target_exe automatically.
                    let _ = &target_exe;
                    std::process::exit(0);
                }
                AppEvent::LauncherUpdateFailed { error } => {
                    self.launcher_update_state = LauncherSelfUpdateState::Error(error.clone());
                    self.set_temporary_status(error);
                }
                AppEvent::DiscoveryError { error, .. } => match error {
                    NetError::RateLimited => {
                        let msg = self.i18n.text("error-rate-limit");
                        self.set_temporary_status(msg);
                    }
                    NetError::Offline(_) => {
                        let msg = self.i18n.text("error-offline");
                        self.set_temporary_status(msg);
                    }
                    _ => {}
                },
                AppEvent::DownloadProgress {
                    app_id,
                    progress,
                    bytes_downloaded,
                    total_bytes,
                } => {
                    self.states.insert(
                        app_id,
                        AppInstallState::Downloading {
                            progress,
                            bytes_downloaded,
                            total_bytes,
                        },
                    );
                }
                AppEvent::StateChange { app_id, new_state } => {
                    self.states.insert(app_id, new_state);
                }
                AppEvent::OperationSuccess { message, .. } => {
                    self.set_temporary_status(message);
                }
                AppEvent::OperationFailed { app_id, error } => {
                    self.set_temporary_status(error.clone());
                    self.states
                        .insert(app_id, AppInstallState::Error { message: error });
                }
            }
        }

        if completed_count > 0 && self.is_checking_releases {
            self.is_checking_releases = false;
        }
    }
}

impl eframe::App for PrismApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_incoming_events();
        self.poll_process_status();

        if let Some((_, created)) = self.status_message {
            if created.elapsed().as_secs() > 4 {
                self.status_message = None;
            }
        }

        // Lazy load authentic application icon textures
        if self.app_textures.is_empty() {
            let app_icons: [(AppId, &str, &[u8]); 4] = [
                (
                    AppId::Trim,
                    "trim_icon",
                    include_bytes!("../../../assets/apps/trim.rgba"),
                ),
                (
                    AppId::Stem,
                    "stem_icon",
                    include_bytes!("../../../assets/apps/stem.rgba"),
                ),
                (
                    AppId::Beam,
                    "beam_icon",
                    include_bytes!("../../../assets/apps/beam.rgba"),
                ),
                (
                    AppId::Dram,
                    "dram_icon",
                    include_bytes!("../../../assets/apps/dram.rgba"),
                ),
            ];
            for (id, name, bytes) in app_icons {
                let img = egui::ColorImage::from_rgba_unmultiplied([256, 256], bytes);
                let tex = ctx.load_texture(name, img, Default::default());
                self.app_textures.insert(id, tex);
            }
        }

        crate::ui::render_app_ui(self, ctx);

        let has_active_transfer = self.is_checking_releases
            || matches!(
                self.launcher_update_state,
                LauncherSelfUpdateState::Updating { .. }
            )
            || self.states.values().any(|s| {
                matches!(
                    s,
                    AppInstallState::Downloading { .. }
                        | AppInstallState::Verifying
                        | AppInstallState::Installing
                )
            });

        let has_running_or_status = self.status_message.is_some()
            || self
                .states
                .values()
                .any(|s| matches!(s, AppInstallState::Running));

        if has_active_transfer {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else if has_running_or_status {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

impl PrismApp {
    pub(crate) fn app_counts(&self) -> (usize, usize, usize) {
        let all = self.catalog.all().len();
        let mut installed = 0;
        let mut updates = 0;
        for app in self.catalog.all() {
            if let Some(state) = self.states.get(&app.id) {
                match state {
                    AppInstallState::Installed { .. } | AppInstallState::Running => {
                        installed += 1;
                    }
                    AppInstallState::UpdateAvailable { .. } => {
                        installed += 1;
                        updates += 1;
                    }
                    _ => {}
                }
            }
        }
        (all, installed, updates)
    }

    pub(crate) fn filtered_apps(&self) -> Vec<prism_core::AppEntry> {
        let apps = self.catalog.all().to_vec();
        match self.app_filter {
            AppFilter::All => apps,
            AppFilter::Installed => apps
                .into_iter()
                .filter(|a| {
                    if let Some(state) = self.states.get(&a.id) {
                        matches!(
                            state,
                            AppInstallState::Installed { .. }
                                | AppInstallState::Running
                                | AppInstallState::UpdateAvailable { .. }
                        )
                    } else {
                        false
                    }
                })
                .collect(),
            AppFilter::Updates => apps
                .into_iter()
                .filter(|a| {
                    if let Some(state) = self.states.get(&a.id) {
                        matches!(state, AppInstallState::UpdateAvailable { .. })
                    } else {
                        false
                    }
                })
                .collect(),
        }
    }

    pub(crate) fn update_all_apps(&mut self) {
        let apps = self.catalog.all().to_vec();
        for app in apps {
            if let Some(AppInstallState::UpdateAvailable { latest_release, .. }) =
                self.states.get(&app.id).cloned()
            {
                self.start_install_pipeline(app.id, latest_release);
            }
        }
    }
}

fn find_local_project_binary(app_id: AppId) -> Option<PathBuf> {
    let (folder, bin_name) = match app_id {
        AppId::Trim => ("triM", "trim"),
        AppId::Stem => ("steM", "stem"),
        AppId::Beam => ("beaM", "beam"),
        AppId::Dram => ("draM", "dram"),
    };

    if let Ok(home) = std::env::var("HOME") {
        let base = std::path::Path::new(&home)
            .join("Projeler")
            .join(folder)
            .join("target");
        let release_bin = base.join("release").join(bin_name);
        if release_bin.exists() {
            return Some(release_bin);
        }
        let debug_bin = base.join("debug").join(bin_name);
        if debug_bin.exists() {
            return Some(debug_bin);
        }
    }

    None
}
