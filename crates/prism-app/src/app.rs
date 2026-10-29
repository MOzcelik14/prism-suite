use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender};
use egui::{
    vec2, Align, Align2, Button, Checkbox, Color32, DragValue, Layout, ProgressBar, RichText,
    Rounding, Stroke, Ui, Window,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use prism_core::branding::{
    GITHUB_ORG_URL, LAUNCHER_DISPLAY_NAME, LAUNCHER_REPO_URL, SUITE_DISPLAY_NAME, SUITE_LICENSE,
    SUITE_VERSION,
};
use prism_core::catalog::{AppCatalog, AppId};
use prism_core::config::{LauncherConfig, UpdateFrequency};
use prism_core::shared::SharedPreferences;
use prism_core::state::{AppInstallState, ReleaseChannel, ReleaseInfo};
use prism_i18n::{I18n, Language};
use prism_install::InstallationManager;
use prism_net::{verify_ed25519_signature, verify_sha256sums, NetError, NetworkClient};
use semver::Version;

use crate::icons::{draw_app_icon, draw_suite_logo};
use crate::theme::{
    ACCENT_AMBER, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_INDIGO, ACCENT_VIOLET, BG_CARD, BG_SECONDARY,
    BORDER_COLOR, TEXT_MUTED, TEXT_PRIMARY,
};

/// Navigation tabs in the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Apps,
    WhatsNew,
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
    },
    LauncherUpdateFailed {
        error: String,
    },
}

/// Main application structure for prisM. Suite launcher.
pub struct PrismApp {
    catalog: AppCatalog,
    i18n: I18n,
    config: LauncherConfig,
    install_mgr: InstallationManager,
    net_client: NetworkClient,
    tokio_runtime: tokio::runtime::Runtime,
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
    markdown_cache: CommonMarkCache,
    states: HashMap<AppId, AppInstallState>,
    running_pids: HashMap<AppId, u32>,
    cancel_tokens: HashMap<AppId, Arc<AtomicBool>>,
    selected_detail_app: Option<AppId>,
    uninstall_target_app: Option<AppId>,
    uninstall_remove_data: bool,
    show_settings: bool,
    show_about: bool,
    is_checking_releases: bool,
    last_process_poll: std::time::Instant,
    status_message: Option<(String, std::time::Instant)>,
    active_tab: ActiveTab,
    launcher_update_state: LauncherSelfUpdateState,
    dismissed_launcher_update: bool,
    discovered_releases: Vec<(String, ReleaseInfo)>,
    custom_dir_input: String,
}

impl PrismApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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

        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        // Write shared preferences for child applications
        let shared_prefs = SharedPreferences {
            language: i18n.language().as_code().to_string(),
            launcher_version: SUITE_VERSION.to_string(),
        };
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
            launcher_update_state: LauncherSelfUpdateState::Idle,
            dismissed_launcher_update: false,
            discovered_releases: Vec::new(),
            custom_dir_input,
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
                Ok(()) => {
                    let _ = tx.send(AppEvent::LauncherUpdated {
                        version: release.version.to_string(),
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

    fn set_temporary_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
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
                AppEvent::LauncherUpdated { version } => {
                    self.launcher_update_state = LauncherSelfUpdateState::Updated {
                        version: version.clone(),
                    };
                    self.set_temporary_status(self.i18n.text_with_arg(
                        "msg-launcher-updated",
                        "version",
                        &version,
                    ));
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

        // Top Navigation Header
        egui::TopBottomPanel::top("top_header")
            .frame(
                egui::Frame::none()
                    .fill(BG_SECONDARY)
                    .inner_margin(egui::Margin::symmetric(24.0_f32, 16.0_f32))
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (logo_rect, _) =
                        ui.allocate_exact_size(vec2(36.0_f32, 36.0_f32), egui::Sense::hover());
                    draw_suite_logo(ui.painter(), logo_rect);

                    ui.add_space(8.0_f32);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(LAUNCHER_DISPLAY_NAME)
                                    .size(18.0_f32)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new("Suite")
                                    .size(18.0_f32)
                                    .strong()
                                    .color(ACCENT_INDIGO),
                            );
                        });
                        ui.label(
                            RichText::new(self.i18n.text("suite-subtitle"))
                                .size(11.0_f32)
                                .color(TEXT_MUTED),
                        );
                    });

                    ui.add_space(32.0_f32);
                    ui.horizontal(|ui| {
                        let apps_tab = ui.selectable_label(
                            self.active_tab == ActiveTab::Apps,
                            RichText::new(self.i18n.text("nav-apps")).size(14.0_f32),
                        );
                        if apps_tab.clicked() {
                            self.active_tab = ActiveTab::Apps;
                        }
                        let whats_new_tab = ui.selectable_label(
                            self.active_tab == ActiveTab::WhatsNew,
                            RichText::new(self.i18n.text("nav-whats-new")).size(14.0_f32),
                        );
                        if whats_new_tab.clicked() {
                            self.active_tab = ActiveTab::WhatsNew;
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(self.i18n.text("action-about")).size(13.0_f32),
                                )
                                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                                .rounding(Rounding::same(6.0_f32)),
                            )
                            .clicked()
                        {
                            self.show_about = true;
                        }

                        if ui
                            .add(
                                Button::new(
                                    RichText::new(self.i18n.text("action-settings")).size(13.0_f32),
                                )
                                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                                .rounding(Rounding::same(6.0_f32)),
                            )
                            .clicked()
                        {
                            self.show_settings = true;
                        }

                        let refresh_btn = Button::new(
                            RichText::new(if self.is_checking_releases {
                                "...".to_string()
                            } else {
                                self.i18n.text("action-refresh")
                            })
                            .size(13.0_f32),
                        )
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                        .rounding(Rounding::same(6.0_f32));

                        if ui
                            .add_enabled(!self.is_checking_releases, refresh_btn)
                            .clicked()
                        {
                            self.check_for_updates(true);
                            self.set_temporary_status("Checking for updates...".to_string());
                        }
                    });
                });
            });

        // Launcher self-update banner
        match &self.launcher_update_state {
            LauncherSelfUpdateState::UpdateAvailable(rel) if !self.dismissed_launcher_update => {
                let rel_clone = (**rel).clone();
                egui::TopBottomPanel::top("launcher_update_banner")
                    .frame(
                        egui::Frame::none()
                            .fill(Color32::from_rgb(30, 41, 59))
                            .inner_margin(egui::Margin::symmetric(24.0_f32, 10.0_f32))
                            .stroke(Stroke::new(1.0_f32, ACCENT_INDIGO)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✨").size(15.0_f32));
                            ui.label(
                                RichText::new(self.i18n.text_with_arg(
                                    "banner-update-available",
                                    "version",
                                    &rel_clone.version.to_string(),
                                ))
                                .strong()
                                .color(TEXT_PRIMARY),
                            );

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button(self.i18n.text("action-later")).clicked() {
                                    self.dismissed_launcher_update = true;
                                }
                                if ui.button(self.i18n.text("action-view-notes")).clicked() {
                                    self.active_tab = ActiveTab::WhatsNew;
                                }
                                let update_btn = Button::new(
                                    RichText::new(self.i18n.text("action-update-now"))
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(ACCENT_INDIGO)
                                .rounding(Rounding::same(4.0_f32));

                                if ui.add(update_btn).clicked() {
                                    self.trigger_launcher_self_update(rel_clone);
                                }
                            });
                        });
                    });
            }
            LauncherSelfUpdateState::Updating { progress } => {
                let prog = *progress;
                egui::TopBottomPanel::top("launcher_updating_banner")
                    .frame(
                        egui::Frame::none()
                            .fill(Color32::from_rgb(30, 41, 59))
                            .inner_margin(egui::Margin::symmetric(24.0_f32, 10.0_f32))
                            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(self.i18n.text("status-launcher-updating"))
                                    .color(ACCENT_CYAN),
                            );
                            ui.add(ProgressBar::new(prog).desired_width(200.0_f32));
                        });
                    });
            }
            LauncherSelfUpdateState::Updated { version } => {
                let ver = version.clone();
                egui::TopBottomPanel::top("launcher_updated_banner")
                    .frame(
                        egui::Frame::none()
                            .fill(Color32::from_rgb(6, 78, 59))
                            .inner_margin(egui::Margin::symmetric(24.0_f32, 10.0_f32))
                            .stroke(Stroke::new(1.0_f32, ACCENT_EMERALD)),
                    )
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("✓").strong().color(ACCENT_EMERALD));
                            ui.label(
                                RichText::new(self.i18n.text_with_arg(
                                    "msg-launcher-updated",
                                    "version",
                                    &ver,
                                ))
                                .color(Color32::WHITE),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button(self.i18n.text("action-close")).clicked() {
                                    self.launcher_update_state = LauncherSelfUpdateState::Idle;
                                }
                            });
                        });
                    });
            }
            _ => {}
        }

        // Bottom status footer if message exists
        if let Some((ref msg, _)) = self.status_message {
            egui::TopBottomPanel::bottom("status_footer")
                .frame(
                    egui::Frame::none()
                        .fill(BG_CARD)
                        .inner_margin(egui::Margin::symmetric(20.0_f32, 8.0_f32))
                        .stroke(Stroke::new(1.0_f32, BORDER_COLOR)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ℹ ").color(ACCENT_CYAN));
                        ui.label(RichText::new(msg).size(12.0_f32).color(TEXT_PRIMARY));
                    });
                });
        }

        // Central Content Area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                ActiveTab::Apps => {
                    ui.add_space(16.0_f32);

                    ui.horizontal(|ui| {
                        ui.add_space(24.0_f32);
                        ui.label(
                            RichText::new(self.i18n.text("nav-apps"))
                                .size(20.0_f32)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    });

                    ui.add_space(12.0_f32);

                    let apps = self.catalog.all().to_vec();
                    for app in &apps {
                        ui.horizontal(|ui| {
                            ui.add_space(24.0_f32);
                            self.render_app_card(ui, app);
                            ui.add_space(24.0_f32);
                        });
                        ui.add_space(14.0_f32);
                    }

                    ui.add_space(20.0_f32);
                }
                ActiveTab::WhatsNew => {
                    self.render_whats_new(ui);
                }
            });
        });

        // App Details Modal
        if let Some(app_id) = self.selected_detail_app {
            self.render_details_window(ctx, app_id);
        }

        // Uninstall Confirmation Dialog
        if let Some(app_id) = self.uninstall_target_app {
            self.render_uninstall_dialog(ctx, app_id);
        }

        // Settings Dialog Window
        if self.show_settings {
            self.render_settings_window(ctx);
        }

        // About Dialog Window
        if self.show_about {
            self.render_about_window(ctx);
        }
    }
}

impl PrismApp {
    fn render_app_card(&mut self, ui: &mut Ui, app: &prism_core::AppEntry) {
        let card_width = ui.available_width();
        let card_height = 114.0_f32;
        let mut clicked_details = false;

        let card_rect = ui
            .allocate_ui(vec2(card_width, card_height), |ui| {
                let current_state =
                    self.states
                        .get(&app.id)
                        .cloned()
                        .unwrap_or(AppInstallState::NotInstalled {
                            latest_release: None,
                        });

                egui::Frame::none()
                    .fill(BG_CARD)
                    .rounding(Rounding::same(10.0_f32))
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .inner_margin(egui::Margin::same(16.0_f32))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (icon_rect, _) = ui.allocate_exact_size(
                                vec2(76.0_f32, 76.0_f32),
                                egui::Sense::hover(),
                            );
                            draw_app_icon(ui.painter(), icon_rect, app.id);

                            ui.add_space(16.0_f32);

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&app.display_name)
                                            .size(20.0_f32)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );

                                    let cat_color = match app.id {
                                        AppId::Trim => ACCENT_CYAN,
                                        AppId::Stem => ACCENT_EMERALD,
                                        AppId::Beam => ACCENT_VIOLET,
                                    };
                                    ui.label(
                                        RichText::new(format!(" {} ", app.category))
                                            .size(11.0_f32)
                                            .color(cat_color),
                                    );

                                    self.render_status_badge(ui, &current_state);
                                });

                                ui.add_space(3.0_f32);

                                ui.label(
                                    RichText::new(self.i18n.text(&app.tagline_key))
                                        .size(13.0_f32)
                                        .color(Color32::from_rgb(209, 213, 219)),
                                );

                                ui.add_space(2.0_f32);

                                if let AppInstallState::Downloading { progress, .. } = current_state
                                {
                                    ui.add(ProgressBar::new(progress).desired_width(280.0_f32));
                                } else {
                                    ui.label(
                                        RichText::new(self.i18n.text(&app.description_key))
                                            .size(11.5_f32)
                                            .color(TEXT_MUTED),
                                    );
                                }
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                self.render_app_actions(ui, app, &current_state);

                                ui.add_space(8.0_f32);

                                let details_btn = Button::new(
                                    RichText::new(self.i18n.text("action-details")).size(12.5_f32),
                                )
                                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                                .rounding(Rounding::same(6.0_f32))
                                .min_size(vec2(76.0_f32, 32.0_f32));

                                if ui.add(details_btn).clicked() {
                                    clicked_details = true;
                                }
                            });
                        });
                    });
            })
            .response
            .rect;

        if clicked_details {
            self.selected_detail_app = Some(app.id);
        }

        if ui.rect_contains_pointer(card_rect) {
            ui.painter().rect_stroke(
                card_rect,
                Rounding::same(10.0_f32),
                Stroke::new(
                    1.0_f32,
                    Color32::from_rgb(99, 102, 241).linear_multiply(0.4_f32),
                ),
            );
        }
    }

    fn render_status_badge(&self, ui: &mut Ui, state: &AppInstallState) {
        match state {
            AppInstallState::NotInstalled { latest_release } => {
                let text = self.i18n.text("status-not-installed");
                let ver_str = latest_release
                    .as_ref()
                    .map(|r| format!(" (v{})", r.version))
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!(" • {}{} ", text, ver_str))
                        .size(11.0_f32)
                        .color(TEXT_MUTED),
                );
            }
            AppInstallState::NoReleaseAvailable => {
                let text = self.i18n.text("status-no-release");
                ui.label(
                    RichText::new(format!(" • {} ", text))
                        .size(11.0_f32)
                        .color(TEXT_MUTED),
                );
            }
            AppInstallState::Installed { version, .. } => {
                let text = self.i18n.text("status-installed");
                ui.label(
                    RichText::new(format!(" • {} v{} ", text, version))
                        .size(11.0_f32)
                        .color(ACCENT_EMERALD),
                );
            }
            AppInstallState::UpdateAvailable { latest_release, .. } => {
                let text = self.i18n.text("status-update-available");
                ui.label(
                    RichText::new(format!(" • {} (v{}) ", text, latest_release.version))
                        .size(11.0_f32)
                        .color(ACCENT_AMBER),
                );
            }
            AppInstallState::Downloading { progress, .. } => {
                let text = self.i18n.text("status-downloading");
                ui.label(
                    RichText::new(format!(" • {} {:.0}% ", text, progress * 100.0_f32))
                        .size(11.0_f32)
                        .color(ACCENT_CYAN),
                );
            }
            AppInstallState::Verifying => {
                let text = self.i18n.text("status-verifying");
                ui.label(
                    RichText::new(format!(" • {} ", text))
                        .size(11.0_f32)
                        .color(ACCENT_CYAN),
                );
            }
            AppInstallState::Installing => {
                let text = self.i18n.text("status-installing");
                ui.label(
                    RichText::new(format!(" • {} ", text))
                        .size(11.0_f32)
                        .color(ACCENT_CYAN),
                );
            }
            AppInstallState::Running => {
                let text = self.i18n.text("status-running");
                ui.label(
                    RichText::new(format!(" • {} ", text))
                        .size(11.0_f32)
                        .color(ACCENT_CYAN),
                );
            }
            AppInstallState::Error { .. } => {
                let text = self.i18n.text("status-error");
                ui.label(
                    RichText::new(format!(" • {} ", text))
                        .size(11.0_f32)
                        .color(Color32::from_rgb(244, 63, 94)),
                );
            }
        }
    }

    fn render_app_actions(
        &mut self,
        ui: &mut Ui,
        app: &prism_core::AppEntry,
        state: &AppInstallState,
    ) {
        match state {
            AppInstallState::NotInstalled { latest_release } => {
                let btn = Button::new(
                    RichText::new(self.i18n.text("action-install"))
                        .size(13.0_f32)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(ACCENT_INDIGO)
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(94.0_f32, 32.0_f32));

                if ui.add(btn).clicked() {
                    if let Some(ref rel) = latest_release {
                        self.start_install_pipeline(app.id, rel.clone());
                    } else {
                        self.states.insert(
                            app.id,
                            AppInstallState::Installed {
                                version: "0.1.0".to_string(),
                                previous_version: None,
                                latest_release: None,
                            },
                        );
                    }
                }
            }
            AppInstallState::NoReleaseAvailable => {
                let btn = Button::new(
                    RichText::new(self.i18n.text("action-install"))
                        .size(13.0_f32)
                        .color(TEXT_MUTED),
                )
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(94.0_f32, 32.0_f32));

                ui.add_enabled(false, btn);
            }
            AppInstallState::Installed { .. } => {
                let launch_btn = Button::new(
                    RichText::new(self.i18n.text("action-launch"))
                        .size(13.0_f32)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(ACCENT_EMERALD.linear_multiply(0.85_f32))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(86.0_f32, 32.0_f32));

                if ui.add(launch_btn).clicked() {
                    self.launch_application(app.id);
                }

                ui.add_space(4.0_f32);

                let uninstall_btn = Button::new(
                    RichText::new(self.i18n.text("action-uninstall"))
                        .size(12.0_f32)
                        .color(TEXT_MUTED),
                )
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(72.0_f32, 32.0_f32));

                if ui.add(uninstall_btn).clicked() {
                    self.uninstall_target_app = Some(app.id);
                }
            }
            AppInstallState::UpdateAvailable { latest_release, .. } => {
                let update_btn = Button::new(
                    RichText::new(self.i18n.text("action-update"))
                        .size(13.0_f32)
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(ACCENT_AMBER)
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(86.0_f32, 32.0_f32));

                let rel_clone = latest_release.clone();
                if ui.add(update_btn).clicked() {
                    self.start_install_pipeline(app.id, rel_clone);
                }

                ui.add_space(4.0_f32);

                let launch_btn = Button::new(
                    RichText::new(self.i18n.text("action-launch"))
                        .size(12.0_f32)
                        .color(TEXT_PRIMARY),
                )
                .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(72.0_f32, 32.0_f32));

                if ui.add(launch_btn).clicked() {
                    self.launch_application(app.id);
                }
            }
            AppInstallState::Downloading { .. } => {
                let cancel_btn = Button::new(
                    RichText::new(self.i18n.text("action-cancel-download"))
                        .size(12.0_f32)
                        .color(Color32::from_rgb(244, 63, 94)),
                )
                .stroke(Stroke::new(
                    1.0_f32,
                    Color32::from_rgb(244, 63, 94).linear_multiply(0.5_f32),
                ))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(86.0_f32, 32.0_f32));

                if ui.add(cancel_btn).clicked() {
                    self.cancel_download(app.id);
                }
            }
            AppInstallState::Verifying | AppInstallState::Installing => {
                ui.label(RichText::new("⏳").size(16.0_f32));
            }
            AppInstallState::Running => {
                let running_label = Button::new(
                    RichText::new(self.i18n.text("action-launch-running"))
                        .size(13.0_f32)
                        .color(ACCENT_CYAN),
                )
                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.5_f32)))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(94.0_f32, 32.0_f32));

                ui.add_enabled(false, running_label);
            }
            AppInstallState::Error { .. } => {
                let retry_btn = Button::new(
                    RichText::new(self.i18n.text("action-retry"))
                        .size(12.0_f32)
                        .color(Color32::WHITE),
                )
                .fill(ACCENT_INDIGO)
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(76.0_f32, 32.0_f32));

                if ui.add(retry_btn).clicked() {
                    self.check_for_updates(true);
                }
            }
        }
    }

    fn render_details_window(&mut self, ctx: &egui::Context, app_id: AppId) {
        let app = match self.catalog.get(app_id) {
            Some(a) => a.clone(),
            None => {
                self.selected_detail_app = None;
                return;
            }
        };

        let mut open = true;
        let current_state =
            self.states
                .get(&app_id)
                .cloned()
                .unwrap_or(AppInstallState::NotInstalled {
                    latest_release: None,
                });

        Window::new(format!(
            "{} — {}",
            app.display_name,
            self.i18n.text("details-title")
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .anchor(Align2::CENTER_CENTER, vec2(0.0_f32, 0.0_f32))
        .default_width(620.0_f32)
        .default_height(520.0_f32)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (icon_rect, _) =
                    ui.allocate_exact_size(vec2(54.0_f32, 54.0_f32), egui::Sense::hover());
                draw_app_icon(ui.painter(), icon_rect, app.id);

                ui.add_space(12.0_f32);

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&app.display_name)
                                .size(22.0_f32)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        self.render_status_badge(ui, &current_state);
                    });
                    ui.label(
                        RichText::new(self.i18n.text(&app.tagline_key))
                            .size(13.0_f32)
                            .color(TEXT_MUTED),
                    );
                });
            });

            ui.add_space(10.0_f32);
            ui.separator();
            ui.add_space(10.0_f32);

            // Metadata block
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("{}:", self.i18n.text("details-repository"))).strong(),
                );
                ui.hyperlink(&app.repository);

                ui.add_space(16.0_f32);

                ui.label(RichText::new(format!("{}:", self.i18n.text("details-branch"))).strong());
                ui.label(RichText::new(&app.default_branch).color(ACCENT_CYAN));
            });

            // Rollback button if previous version exists
            if let Some(prev_ver) = current_state.previous_version() {
                ui.add_space(8.0_f32);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("↩ ").color(ACCENT_CYAN));
                        ui.label(
                            RichText::new(format!(
                                "{} (v{})",
                                self.i18n.text("details-rollback-desc"),
                                prev_ver
                            ))
                            .size(12.0_f32)
                            .color(TEXT_PRIMARY),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let is_running = current_state == AppInstallState::Running;
                            let rollback_btn = Button::new(
                                RichText::new(self.i18n.text("action-rollback")).size(12.0_f32),
                            )
                            .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                            .rounding(Rounding::same(6.0_f32));

                            if ui.add_enabled(!is_running, rollback_btn).clicked() {
                                self.trigger_rollback(app.id);
                            }
                        });
                    });
                });
            }

            // Runtime notes if defined
            if let Some(ref notes_key) = app.runtime_notes_key {
                ui.add_space(8.0_f32);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚙ ").color(ACCENT_AMBER));
                        ui.label(
                            RichText::new(self.i18n.text("details-runtime-notes"))
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    });
                    ui.label(
                        RichText::new(self.i18n.text(notes_key))
                            .size(12.0_f32)
                            .color(TEXT_MUTED),
                    );
                });
            }

            ui.add_space(12.0_f32);
            ui.separator();
            ui.add_space(10.0_f32);

            // Release notes section
            ui.label(
                RichText::new(self.i18n.text("details-release-notes"))
                    .size(16.0_f32)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(6.0_f32);

            let release_notes = current_state
                .latest_release()
                .map(|r| r.release_notes.as_str())
                .unwrap_or("");

            egui::ScrollArea::vertical()
                .max_height(200.0_f32)
                .show(ui, |ui| {
                    if release_notes.trim().is_empty() {
                        ui.label(
                            RichText::new(self.i18n.text("details-no-notes"))
                                .italics()
                                .color(TEXT_MUTED),
                        );
                    } else {
                        CommonMarkViewer::new().show(ui, &mut self.markdown_cache, release_notes);
                    }
                });

            ui.add_space(14.0_f32);
            ui.separator();
            ui.add_space(10.0_f32);

            ui.horizontal(|ui| {
                self.render_app_actions(ui, &app, &current_state);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.i18n.text("action-close")).clicked() {
                        self.selected_detail_app = None;
                    }
                });
            });
        });

        if !open {
            self.selected_detail_app = None;
        }
    }

    fn render_uninstall_dialog(&mut self, ctx: &egui::Context, app_id: AppId) {
        let app = match self.catalog.get(app_id) {
            Some(a) => a.clone(),
            None => {
                self.uninstall_target_app = None;
                return;
            }
        };

        let mut open = true;
        Window::new(self.i18n.text("uninstall-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0_f32, 0.0_f32))
            .default_width(380.0_f32)
            .show(ctx, |ui| {
                ui.add_space(8.0_f32);

                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        self.i18n.text("uninstall-confirm-msg"),
                        app.display_name
                    ))
                    .size(14.0_f32)
                    .color(TEXT_PRIMARY),
                );

                ui.add_space(12.0_f32);

                ui.add(Checkbox::new(
                    &mut self.uninstall_remove_data,
                    self.i18n.text("uninstall-remove-data"),
                ));

                ui.add_space(18.0_f32);

                ui.horizontal(|ui| {
                    let confirm_btn = Button::new(
                        RichText::new(self.i18n.text("uninstall-confirm-btn"))
                            .size(13.0_f32)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(220, 38, 38))
                    .rounding(Rounding::same(6.0_f32));

                    if ui.add(confirm_btn).clicked() {
                        self.execute_uninstall(app_id, self.uninstall_remove_data);
                    }

                    if ui.button(self.i18n.text("action-cancel")).clicked() {
                        self.uninstall_target_app = None;
                    }
                });
            });

        if !open {
            self.uninstall_target_app = None;
        }
    }

    fn render_settings_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_settings;
        Window::new(self.i18n.text("settings-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0_f32, 0.0_f32))
            .default_width(440.0_f32)
            .show(ctx, |ui| {
                ui.add_space(8.0_f32);

                // Language selection
                ui.label(
                    RichText::new(self.i18n.text("settings-language"))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0_f32);

                let current_lang = self.i18n.language();
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            current_lang == Language::English,
                            self.i18n.text("settings-language-en"),
                        )
                        .clicked()
                    {
                        self.i18n.set_language(Language::English);
                        self.config.language = Some("en".to_string());
                        let _ = self.config.save();

                        // Sync shared preferences
                        let shared = SharedPreferences {
                            language: "en".to_string(),
                            launcher_version: SUITE_VERSION.to_string(),
                        };
                        let _ = shared.save();
                    }

                    if ui
                        .selectable_label(
                            current_lang == Language::Turkish,
                            self.i18n.text("settings-language-tr"),
                        )
                        .clicked()
                    {
                        self.i18n.set_language(Language::Turkish);
                        self.config.language = Some("tr".to_string());
                        let _ = self.config.save();

                        // Sync shared preferences
                        let shared = SharedPreferences {
                            language: "tr".to_string(),
                            launcher_version: SUITE_VERSION.to_string(),
                        };
                        let _ = shared.save();
                    }
                });

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Release Channel selection
                ui.label(
                    RichText::new(self.i18n.text("settings-channel"))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0_f32);

                let current_channel = self.config.release_channel;
                let mut channel_changed = false;
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            current_channel == ReleaseChannel::Stable,
                            self.i18n.text("settings-channel-stable"),
                        )
                        .clicked()
                    {
                        self.config.release_channel = ReleaseChannel::Stable;
                        channel_changed = true;
                    }

                    if ui
                        .selectable_label(
                            current_channel == ReleaseChannel::Beta,
                            self.i18n.text("settings-channel-beta"),
                        )
                        .clicked()
                    {
                        self.config.release_channel = ReleaseChannel::Beta;
                        channel_changed = true;
                    }
                });

                if channel_changed {
                    let _ = self.config.save();
                    self.check_for_updates(true);
                }

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Keep versions setting
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(self.i18n.text("settings-keep-versions"))
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    if ui
                        .add(
                            DragValue::new(&mut self.config.keep_versions_count)
                                .range(1..=5)
                                .speed(0.1),
                        )
                        .changed()
                    {
                        let _ = self.config.save();
                    }
                });

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Update Check Frequency
                ui.label(
                    RichText::new(self.i18n.text("settings-check-frequency"))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0_f32);

                let current_freq = self.config.check_frequency;
                let mut freq_changed = false;
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            current_freq == UpdateFrequency::EveryLaunch,
                            self.i18n.text("settings-freq-launch"),
                        )
                        .clicked()
                    {
                        self.config.check_frequency = UpdateFrequency::EveryLaunch;
                        freq_changed = true;
                    }
                    if ui
                        .selectable_label(
                            current_freq == UpdateFrequency::Daily,
                            self.i18n.text("settings-freq-daily"),
                        )
                        .clicked()
                    {
                        self.config.check_frequency = UpdateFrequency::Daily;
                        freq_changed = true;
                    }
                    if ui
                        .selectable_label(
                            current_freq == UpdateFrequency::Weekly,
                            self.i18n.text("settings-freq-weekly"),
                        )
                        .clicked()
                    {
                        self.config.check_frequency = UpdateFrequency::Weekly;
                        freq_changed = true;
                    }
                    if ui
                        .selectable_label(
                            current_freq == UpdateFrequency::Manual,
                            self.i18n.text("settings-freq-manual"),
                        )
                        .clicked()
                    {
                        self.config.check_frequency = UpdateFrequency::Manual;
                        freq_changed = true;
                    }
                });

                if freq_changed {
                    let _ = self.config.save();
                }

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Auto-update toggle
                if ui
                    .checkbox(
                        &mut self.config.auto_update,
                        self.i18n.text("settings-auto-update"),
                    )
                    .changed()
                {
                    let _ = self.config.save();
                }

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Check updates on startup checkbox
                if ui
                    .checkbox(
                        &mut self.config.check_updates_on_launch,
                        self.i18n.text("settings-check-updates-launch"),
                    )
                    .changed()
                {
                    let _ = self.config.save();
                }

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                // Install directory display and custom chooser
                ui.label(
                    RichText::new(self.i18n.text("settings-install-directory"))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0_f32);

                let dir_str = self
                    .install_mgr
                    .apps_root_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "Default system app data directory".to_string());

                ui.label(
                    RichText::new(format!("Current: {}", dir_str))
                        .size(12.0_f32)
                        .color(TEXT_MUTED),
                );
                ui.add_space(4.0_f32);

                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.custom_dir_input);

                    if ui.button(self.i18n.text("action-save")).clicked() {
                        let trimmed = self.custom_dir_input.trim();
                        if trimmed.is_empty() {
                            self.config.custom_install_dir = None;
                        } else {
                            self.config.custom_install_dir = Some(PathBuf::from(trimmed));
                        }
                        let _ = self.config.save();
                        self.install_mgr =
                            InstallationManager::new(self.config.custom_install_dir.clone());
                    }
                });

                ui.add_space(18.0_f32);
                ui.separator();
                ui.add_space(10.0_f32);

                ui.horizontal(|ui| {
                    if ui
                        .button(self.i18n.text("settings-reset-defaults"))
                        .clicked()
                    {
                        self.config.reset_to_defaults();
                        let _ = self.config.save();
                        self.i18n.set_language(Language::English);
                        self.custom_dir_input = String::new();
                        self.install_mgr = InstallationManager::new(None);
                        let shared = SharedPreferences {
                            language: "en".to_string(),
                            launcher_version: SUITE_VERSION.to_string(),
                        };
                        let _ = shared.save();
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(self.i18n.text("action-close")).clicked() {
                            self.show_settings = false;
                        }
                    });
                });
            });
        self.show_settings = open;
    }

    fn render_whats_new(&mut self, ui: &mut Ui) {
        ui.add_space(16.0_f32);
        ui.horizontal(|ui| {
            ui.add_space(24.0_f32);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.i18n.text("whats-new-title"))
                        .size(20.0_f32)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(
                        "Latest release notes and improvements across all suite applications",
                    )
                    .size(12.0_f32)
                    .color(TEXT_MUTED),
                );
            });
        });

        ui.add_space(16.0_f32);

        if self.discovered_releases.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(24.0_f32);
                ui.label(
                    RichText::new(self.i18n.text("whats-new-no-releases"))
                        .italics()
                        .color(TEXT_MUTED),
                );
            });
            return;
        }

        let releases = self.discovered_releases.clone();
        for (app_name, release) in releases {
            ui.horizontal(|ui| {
                ui.add_space(24.0_f32);
                let content_width = (ui.available_width() - 24.0_f32).max(200.0_f32);
                egui::Frame::none()
                    .fill(BG_CARD)
                    .stroke(Stroke::new(1.0_f32, BORDER_COLOR))
                    .rounding(Rounding::same(8.0_f32))
                    .inner_margin(egui::Margin::same(18.0_f32))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&app_name)
                                    .size(16.0_f32)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );

                            ui.label(
                                RichText::new(format!("v{}", release.version))
                                    .size(13.0_f32)
                                    .color(ACCENT_INDIGO),
                            );

                            if let Some(ref date) = release.published_at {
                                let short_date = date.split('T').next().unwrap_or(date);
                                ui.label(
                                    RichText::new(short_date).size(12.0_f32).color(TEXT_MUTED),
                                );
                            }
                        });

                        ui.add_space(8.0_f32);
                        ui.separator();
                        ui.add_space(8.0_f32);

                        if release.release_notes.trim().is_empty() {
                            ui.label(
                                RichText::new(self.i18n.text("details-no-notes"))
                                    .italics()
                                    .color(TEXT_MUTED),
                            );
                        } else {
                            CommonMarkViewer::new().show(
                                ui,
                                &mut self.markdown_cache,
                                &release.release_notes,
                            );
                        }
                    });
            });
            ui.add_space(12.0_f32);
        }
    }

    fn render_about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        Window::new(self.i18n.text("about-title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0_f32, 0.0_f32))
            .default_width(420.0_f32)
            .show(ctx, |ui| {
                ui.add_space(8.0_f32);

                ui.vertical_centered(|ui| {
                    let (logo_rect, _) =
                        ui.allocate_exact_size(vec2(64.0_f32, 64.0_f32), egui::Sense::hover());
                    draw_suite_logo(ui.painter(), logo_rect);

                    ui.add_space(10.0_f32);

                    ui.label(
                        RichText::new(SUITE_DISPLAY_NAME)
                            .size(22.0_f32)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );

                    ui.label(
                        RichText::new(self.i18n.text("about-tagline"))
                            .size(12.5_f32)
                            .color(TEXT_MUTED),
                    );

                    ui.add_space(6.0_f32);

                    ui.label(
                        RichText::new(format!(
                            "{}: v{}",
                            self.i18n.text("about-version"),
                            SUITE_VERSION
                        ))
                        .size(12.0_f32)
                        .color(ACCENT_CYAN),
                    );

                    ui.label(
                        RichText::new(format!(
                            "{}: {}",
                            self.i18n.text("about-license"),
                            SUITE_LICENSE
                        ))
                        .size(12.0_f32)
                        .color(TEXT_MUTED),
                    );
                });

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(14.0_f32);

                ui.label(
                    RichText::new(self.i18n.text("about-source-code"))
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.hyperlink_to("GitHub Repository", LAUNCHER_REPO_URL);
                ui.hyperlink_to("Creative Suite Organization", GITHUB_ORG_URL);

                ui.add_space(14.0_f32);
                ui.separator();
                ui.add_space(10.0_f32);

                ui.label(
                    RichText::new(self.i18n.text("about-copyright"))
                        .size(11.0_f32)
                        .color(TEXT_MUTED),
                );

                ui.add_space(14.0_f32);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.i18n.text("action-close")).clicked() {
                        self.show_about = false;
                    }
                });
            });
        self.show_about = open;
    }
}
