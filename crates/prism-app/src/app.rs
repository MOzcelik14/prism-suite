use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};
use egui::{vec2, Align, Align2, Button, Color32, Layout, RichText, Rounding, Stroke, Ui, Window};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use prism_core::branding::{
    GITHUB_ORG_URL, LAUNCHER_DISPLAY_NAME, LAUNCHER_REPO_URL, SUITE_DISPLAY_NAME, SUITE_LICENSE,
    SUITE_VERSION,
};
use prism_core::catalog::{AppCatalog, AppId};
use prism_core::config::LauncherConfig;
use prism_core::state::{AppInstallState, ReleaseChannel, ReleaseInfo};
use prism_i18n::{I18n, Language};
use prism_install::InstallationManager;
use prism_net::{NetError, NetworkClient};
use semver::Version;

use crate::icons::{draw_app_icon, draw_suite_logo};
use crate::theme::{
    ACCENT_AMBER, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_INDIGO, ACCENT_VIOLET, BG_CARD, BG_SECONDARY,
    BORDER_COLOR, TEXT_MUTED, TEXT_PRIMARY,
};

/// Events sent from background network tasks to the UI thread.
pub enum AppEvent {
    ReleaseDiscovered {
        app_id: AppId,
        release: Option<ReleaseInfo>,
    },
    DiscoveryError {
        _app_id: AppId,
        error: NetError,
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
    selected_detail_app: Option<AppId>,
    show_settings: bool,
    show_about: bool,
    is_checking_releases: bool,
    status_message: Option<(String, std::time::Instant)>,
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

        // Initial states: detect installed state from filesystem
        let mut states = HashMap::new();
        for app in catalog.all() {
            let detected = install_mgr.detect_state(app);
            states.insert(app.id, detected);
        }

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
            selected_detail_app: None,
            show_settings: false,
            show_about: false,
            is_checking_releases: false,
            status_message: None,
        };

        // If configured, trigger background release check on launch
        if app.config.check_updates_on_launch {
            app.check_for_updates(false);
        }

        app
    }

    /// Spawns background worker tasks to fetch remote releases without blocking the UI.
    pub fn check_for_updates(&mut self, force_refresh: bool) {
        self.is_checking_releases = true;
        let apps = self.catalog.all().to_vec();
        let channel = self.config.release_channel;

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
    }

    fn set_temporary_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
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
                        AppInstallState::Installed { version, .. } => {
                            if let Some(ref rel) = release {
                                let inst_ver = Version::parse(&version).ok();
                                if let Some(iv) = inst_ver {
                                    if rel.version > iv {
                                        AppInstallState::UpdateAvailable {
                                            current_version: version,
                                            latest_release: rel.clone(),
                                        }
                                    } else {
                                        AppInstallState::Installed {
                                            version,
                                            latest_release: Some(rel.clone()),
                                        }
                                    }
                                } else {
                                    AppInstallState::Installed {
                                        version,
                                        latest_release: Some(rel.clone()),
                                    }
                                }
                            } else {
                                AppInstallState::Installed {
                                    version,
                                    latest_release: None,
                                }
                            }
                        }
                        AppInstallState::UpdateAvailable {
                            current_version, ..
                        } => {
                            if let Some(rel) = release {
                                AppInstallState::UpdateAvailable {
                                    current_version,
                                    latest_release: rel,
                                }
                            } else {
                                AppInstallState::Installed {
                                    version: current_version,
                                    latest_release: None,
                                }
                            }
                        }
                        AppInstallState::Running => current,
                        _ => {
                            // Not installed
                            if let Some(rel) = release {
                                AppInstallState::NotInstalled {
                                    latest_release: Some(rel),
                                }
                            } else {
                                AppInstallState::NoReleaseAvailable
                            }
                        }
                    };

                    self.states.insert(app_id, new_state);
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
            }
        }

        if completed_count > 0 && self.is_checking_releases {
            self.is_checking_releases = false;
        }
    }
}

impl eframe::App for PrismApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll incoming network events
        self.handle_incoming_events();

        // Clear status message after 4 seconds
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
                    // Logo
                    let (logo_rect, _) =
                        ui.allocate_exact_size(vec2(36.0_f32, 36.0_f32), egui::Sense::hover());
                    draw_suite_logo(ui.painter(), logo_rect);

                    ui.add_space(8.0_f32);

                    // Title & Subtitle
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

                    // Right header action buttons
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
            egui::ScrollArea::vertical().show(ui, |ui| {
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

                // Render each app card
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
            });
        });

        // App Details Modal
        if let Some(app_id) = self.selected_detail_app {
            self.render_details_window(ctx, app_id);
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
        let card_height = 112.0_f32;
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
                            // Vector Icon (76x76)
                            let (icon_rect, _) = ui.allocate_exact_size(
                                vec2(76.0_f32, 76.0_f32),
                                egui::Sense::hover(),
                            );
                            draw_app_icon(ui.painter(), icon_rect, app.id);

                            ui.add_space(16.0_f32);

                            // App Info (Name from catalog, tagline, description)
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    // App name always from catalog
                                    ui.label(
                                        RichText::new(&app.display_name)
                                            .size(20.0_f32)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );

                                    // Category badge
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

                                    // Status Badge
                                    self.render_status_badge(ui, &current_state);
                                });

                                ui.add_space(3.0_f32);

                                // Localized Tagline
                                ui.label(
                                    RichText::new(self.i18n.text(&app.tagline_key))
                                        .size(13.0_f32)
                                        .color(Color32::from_rgb(209, 213, 219)),
                                );

                                ui.add_space(2.0_f32);

                                // Localized Description
                                ui.label(
                                    RichText::new(self.i18n.text(&app.description_key))
                                        .size(11.5_f32)
                                        .color(TEXT_MUTED),
                                );
                            });

                            // Action buttons right-aligned
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                self.render_app_actions(ui, app, &current_state);

                                ui.add_space(8.0_f32);

                                // Details button
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

        // Subtle hover highlight
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

                let target_v = latest_release
                    .as_ref()
                    .map(|r| r.version.to_string())
                    .unwrap_or_else(|| "0.1.0".to_string());

                if ui.add(btn).clicked() {
                    self.states.insert(
                        app.id,
                        AppInstallState::Installed {
                            version: target_v.clone(),
                            latest_release: latest_release.clone(),
                        },
                    );
                    self.set_temporary_status(format!(
                        "{} v{} marked as installed.",
                        app.display_name, target_v
                    ));
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
            AppInstallState::Installed {
                version,
                latest_release,
            } => {
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
                    self.states.insert(app.id, AppInstallState::Running);
                    self.set_temporary_status(format!(
                        "Launching {} v{}...",
                        app.display_name, version
                    ));
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
                    self.states.insert(
                        app.id,
                        AppInstallState::NotInstalled {
                            latest_release: latest_release.clone(),
                        },
                    );
                    self.set_temporary_status(format!("{} uninstalled.", app.display_name));
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

                let app_name = app.display_name.clone();
                let latest_v = latest_release.version.to_string();
                if ui.add(update_btn).clicked() {
                    self.states.insert(
                        app.id,
                        AppInstallState::Installed {
                            version: latest_v.clone(),
                            latest_release: Some(latest_release.clone()),
                        },
                    );
                    self.set_temporary_status(format!("Updated {} to v{}.", app_name, latest_v));
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
                    self.states.insert(app.id, AppInstallState::Running);
                }
            }
            AppInstallState::Running => {
                let stop_btn = Button::new(
                    RichText::new(self.i18n.text("status-running"))
                        .size(13.0_f32)
                        .color(ACCENT_CYAN),
                )
                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.5_f32)))
                .rounding(Rounding::same(6.0_f32))
                .min_size(vec2(94.0_f32, 32.0_f32));

                if ui.add(stop_btn).clicked() {
                    self.states.insert(
                        app.id,
                        AppInstallState::Installed {
                            version: "0.1.0".to_string(),
                            latest_release: None,
                        },
                    );
                }
            }
            _ => {
                ui.label(RichText::new("...").color(TEXT_MUTED));
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
        .default_height(500.0_f32)
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
                .max_height(220.0_f32)
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
                    }
                });

                ui.add_space(16.0_f32);
                ui.separator();
                ui.add_space(16.0_f32);

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

                ui.add_space(16.0_f32);
                ui.separator();
                ui.add_space(16.0_f32);

                // Install directory display
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

                ui.label(RichText::new(dir_str).size(12.0_f32).color(TEXT_MUTED));

                ui.add_space(16.0_f32);
                ui.separator();
                ui.add_space(16.0_f32);

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

                ui.add_space(20.0_f32);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(self.i18n.text("action-close")).clicked() {
                        self.show_settings = false;
                    }
                });
            });
        self.show_settings = open;
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
