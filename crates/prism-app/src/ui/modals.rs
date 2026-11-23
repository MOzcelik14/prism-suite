// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;

use egui::{
    vec2, Align, Align2, Button, Checkbox, Color32, Layout, ProgressBar, RichText, Rounding, Sense,
    Stroke, Ui, Window,
};
use egui_commonmark::CommonMarkViewer;
use prism_core::branding::{
    GITHUB_ORG_URL, LAUNCHER_REPO_URL, SUITE_DISPLAY_NAME, SUITE_LICENSE, SUITE_VERSION,
};
use prism_core::catalog::AppId;
use prism_core::config::UpdateFrequency;
use prism_core::state::{AppInstallState, ReleaseChannel};
use prism_i18n::Language;
use prism_install::InstallationManager;

use crate::app::PrismApp;
use crate::icons::{draw_app_icon, draw_suite_logo};
use crate::theme::{
    ACCENT_AMBER, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_INDIGO, ACCENT_RED, BG_SURFACE,
    BG_SURFACE_ELEVATED, BORDER_STRONG, BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::apps_view::render_card_status_capsule;
use crate::ui::vector_icons::{draw_rollback_icon, draw_settings_icon};

pub fn render_details_window(app: &mut PrismApp, ctx: &egui::Context, app_id: AppId) {
    let entry = match app.catalog.get(app_id) {
        Some(a) => a.clone(),
        None => {
            app.selected_detail_app = None;
            return;
        }
    };

    let mut open = true;
    let mut request_close = false;
    let current_state = app
        .states
        .get(&app_id)
        .cloned()
        .unwrap_or(AppInstallState::NotInstalled {
            latest_release: None,
        });

    Window::new(format!(
        "{} — {}",
        entry.display_name,
        app.i18n.text("details-title")
    ))
    .open(&mut open)
    .collapsible(false)
    .resizable(true)
    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
    .default_width(640.0)
    .default_height(540.0)
    .show(ctx, |ui| {
        ui.horizontal(|ui| {
            let icon_size = vec2(52.0, 52.0);
            if let Some(tex) = app.app_textures.get(&entry.id) {
                ui.add(egui::Image::new((tex.id(), icon_size)).rounding(Rounding::same(10.0)));
            } else {
                let (icon_rect, _) = ui.allocate_exact_size(icon_size, Sense::hover());
                draw_app_icon(ui.painter(), icon_rect, entry.id);
            }

            ui.add_space(12.0);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&entry.display_name)
                            .size(20.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    render_card_status_capsule(app, ui, &current_state);
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(app.i18n.text(&entry.tagline_key))
                        .size(12.5)
                        .color(TEXT_MUTED),
                );
            });
        });

        ui.add_space(12.0);

        // Metadata block
        egui::Frame::none()
            .fill(BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}:", app.i18n.text("details-repository")))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                    ui.hyperlink_to(
                        RichText::new(&entry.repository)
                            .size(12.0)
                            .color(ACCENT_CYAN),
                        &entry.repository,
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(&entry.default_branch)
                                .size(12.0)
                                .strong()
                                .color(ACCENT_CYAN),
                        );
                        ui.label(
                            RichText::new(format!("{}:", app.i18n.text("details-branch")))
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                    });
                });
            });

        // Rollback bar if previous version exists
        if let Some(prev_ver) = current_state.previous_version() {
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(BG_SURFACE)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        draw_rollback_icon(ui.painter(), icon_rect, ACCENT_CYAN);
                        ui.label(
                            RichText::new(format!(
                                "{} (v{})",
                                app.i18n.text("details-rollback-desc"),
                                prev_ver
                            ))
                            .size(12.0)
                            .color(TEXT_PRIMARY),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let is_running = current_state == AppInstallState::Running;
                            let rollback_btn = Button::new(
                                RichText::new(app.i18n.text("action-rollback"))
                                    .size(11.5)
                                    .color(TEXT_PRIMARY),
                            )
                            .fill(BG_SURFACE_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                            .rounding(Rounding::same(6.0));

                            if ui.add_enabled(!is_running, rollback_btn).clicked() {
                                app.trigger_rollback(entry.id);
                            }
                        });
                    });
                });
        }

        // Runtime notes
        if let Some(ref notes_key) = entry.runtime_notes_key {
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(BG_SURFACE)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                        draw_settings_icon(ui.painter(), icon_rect, ACCENT_AMBER);
                        ui.label(
                            RichText::new(app.i18n.text("details-runtime-notes"))
                                .size(12.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    });
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(app.i18n.text(notes_key))
                            .size(11.5)
                            .color(TEXT_MUTED),
                    );
                });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new(app.i18n.text("details-release-notes"))
                .size(14.5)
                .strong()
                .color(TEXT_PRIMARY),
        );
        ui.add_space(6.0);

        let release_notes = current_state
            .latest_release()
            .map(|r| r.release_notes.as_str())
            .unwrap_or("");

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                if release_notes.trim().is_empty() {
                    ui.label(
                        RichText::new(app.i18n.text("details-no-notes"))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                } else {
                    CommonMarkViewer::new().show(ui, &mut app.markdown_cache, release_notes);
                }
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            render_modal_app_actions(app, ui, &entry, &current_state);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let close_btn = Button::new(
                    RichText::new(app.i18n.text("action-close"))
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                )
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(76.0, 32.0));

                if ui.add(close_btn).clicked() {
                    request_close = true;
                }
            });
        });
    });

    if request_close {
        open = false;
    }
    if !open {
        app.selected_detail_app = None;
    }
}

fn render_modal_app_actions(
    app: &mut PrismApp,
    ui: &mut Ui,
    entry: &prism_core::AppEntry,
    state: &AppInstallState,
) {
    match state {
        AppInstallState::NotInstalled { latest_release } => {
            if let Some(ref rel) = latest_release {
                let btn = Button::new(
                    RichText::new(app.i18n.text("action-install"))
                        .size(12.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(ACCENT_INDIGO)
                .rounding(Rounding::same(6.0))
                .min_size(vec2(104.0, 32.0));

                if ui.add(btn).clicked() {
                    app.start_install_pipeline(entry.id, rel.clone());
                }
            } else {
                let btn = Button::new(
                    RichText::new(app.i18n.text("action-source-repo"))
                        .size(12.0)
                        .color(ACCENT_CYAN),
                )
                .fill(ACCENT_CYAN.linear_multiply(0.10))
                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.35)))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(120.0, 32.0));

                if ui.add(btn).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&entry.repository));
                }
            }
        }
        AppInstallState::NoReleaseAvailable => {
            let btn = Button::new(
                RichText::new(app.i18n.text("action-source-repo"))
                    .size(12.0)
                    .color(ACCENT_CYAN),
            )
            .fill(ACCENT_CYAN.linear_multiply(0.10))
            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.35)))
            .rounding(Rounding::same(6.0))
            .min_size(vec2(120.0, 32.0));

            if ui.add(btn).clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&entry.repository));
            }
        }
        AppInstallState::Installed { .. } => {
            let launch_btn = Button::new(
                RichText::new(app.i18n.text("action-launch"))
                    .size(12.5)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT_EMERALD)
            .rounding(Rounding::same(6.0))
            .min_size(vec2(100.0, 32.0));

            if ui.add(launch_btn).clicked() {
                app.launch_application(entry.id);
            }

            let uninstall_btn = Button::new(
                RichText::new(app.i18n.text("action-uninstall"))
                    .size(12.0)
                    .color(ACCENT_RED),
            )
            .fill(BG_SURFACE_ELEVATED)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(6.0))
            .min_size(vec2(84.0, 32.0));

            if ui.add(uninstall_btn).clicked() {
                app.uninstall_target_app = Some(entry.id);
                app.uninstall_remove_data = false;
            }
        }
        AppInstallState::UpdateAvailable { latest_release, .. } => {
            let update_btn = Button::new(
                RichText::new(app.i18n.text("action-update"))
                    .size(12.5)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT_AMBER)
            .rounding(Rounding::same(6.0))
            .min_size(vec2(100.0, 32.0));

            let rel_clone = latest_release.clone();
            if ui.add(update_btn).clicked() {
                app.start_install_pipeline(entry.id, rel_clone);
            }

            let launch_btn = Button::new(
                RichText::new(app.i18n.text("action-launch"))
                    .size(12.0)
                    .color(TEXT_PRIMARY),
            )
            .fill(BG_SURFACE_ELEVATED)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(6.0))
            .min_size(vec2(84.0, 32.0));

            if ui.add(launch_btn).clicked() {
                app.launch_application(entry.id);
            }

            let uninstall_btn = Button::new(
                RichText::new(app.i18n.text("action-uninstall"))
                    .size(12.0)
                    .color(ACCENT_RED),
            )
            .fill(BG_SURFACE_ELEVATED)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(6.0))
            .min_size(vec2(84.0, 32.0));

            if ui.add(uninstall_btn).clicked() {
                app.uninstall_target_app = Some(entry.id);
                app.uninstall_remove_data = false;
            }
        }
        AppInstallState::Downloading { progress, .. } => {
            ui.add(ProgressBar::new(*progress).desired_width(150.0));
            if ui.button(app.i18n.text("action-cancel-download")).clicked() {
                app.cancel_download(entry.id);
            }
        }
        AppInstallState::Verifying => {
            ui.label(
                RichText::new(app.i18n.text("status-verifying"))
                    .size(12.0)
                    .color(ACCENT_CYAN),
            );
        }
        AppInstallState::Installing => {
            ui.label(
                RichText::new(app.i18n.text("status-installing"))
                    .size(12.0)
                    .color(ACCENT_CYAN),
            );
        }
        AppInstallState::Running => {
            let running_btn = Button::new(
                RichText::new(app.i18n.text("action-launch-running"))
                    .size(12.0)
                    .color(ACCENT_EMERALD),
            )
            .fill(ACCENT_EMERALD.linear_multiply(0.12))
            .stroke(Stroke::new(1.0_f32, ACCENT_EMERALD.linear_multiply(0.35)))
            .rounding(Rounding::same(6.0))
            .min_size(vec2(96.0, 32.0));
            ui.add_enabled(false, running_btn);
        }
        AppInstallState::Error { message } => {
            ui.label(RichText::new(message).size(11.5).color(ACCENT_RED));
            if ui.button(app.i18n.text("action-retry")).clicked() {
                app.check_for_updates(true);
            }
        }
    }
}

pub fn render_uninstall_dialog(app: &mut PrismApp, ctx: &egui::Context, app_id: AppId) {
    let entry = match app.catalog.get(app_id) {
        Some(a) => a.clone(),
        None => {
            app.uninstall_target_app = None;
            return;
        }
    };

    let mut open = true;
    Window::new(app.i18n.text("uninstall-title"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .default_width(380.0)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    app.i18n.text("uninstall-confirm-msg"),
                    entry.display_name
                ))
                .size(13.5)
                .color(TEXT_PRIMARY),
            );

            ui.add_space(12.0);
            ui.add(Checkbox::new(
                &mut app.uninstall_remove_data,
                app.i18n.text("uninstall-remove-data"),
            ));

            ui.add_space(16.0);

            ui.horizontal(|ui| {
                let confirm_btn = Button::new(
                    RichText::new(app.i18n.text("uninstall-confirm-btn"))
                        .size(12.5)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(ACCENT_RED)
                .rounding(Rounding::same(6.0))
                .min_size(vec2(90.0, 30.0));

                if ui.add(confirm_btn).clicked() {
                    app.execute_uninstall(app_id, app.uninstall_remove_data);
                }

                let cancel_btn = Button::new(
                    RichText::new(app.i18n.text("action-cancel"))
                        .size(12.5)
                        .color(TEXT_SECONDARY),
                )
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(80.0, 30.0));

                if ui.add(cancel_btn).clicked() {
                    app.uninstall_target_app = None;
                }
            });
        });

    if !open {
        app.uninstall_target_app = None;
    }
}

fn settings_section_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.0_f32)
            .strong()
            .color(TEXT_MUTED),
    );
    ui.add_space(6.0_f32);
}

fn settings_card(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
    egui::Frame::none()
        .fill(BG_SURFACE)
        .rounding(Rounding::same(10.0))
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(16.0_f32, 14.0_f32))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add_contents(ui);
        });
}

fn render_toggle_switch(ui: &mut Ui, checked: &mut bool) -> bool {
    let desired_size = vec2(38.0_f32, 20.0_f32);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
    let changed = response.clicked();
    if changed {
        *checked = !*checked;
    }

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *checked);
        let fill = if *checked {
            ACCENT_INDIGO
        } else {
            BG_SURFACE_ELEVATED
        };
        let stroke = if *checked {
            Stroke::new(1.0_f32, ACCENT_INDIGO)
        } else {
            Stroke::new(1.0_f32, BORDER_STRONG)
        };
        let radius = rect.height() * 0.5_f32;
        ui.painter().rect(rect, radius, fill, stroke);

        let knob_radius = radius - 3.0_f32;
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        ui.painter().circle_filled(
            egui::pos2(knob_x, rect.center().y),
            knob_radius,
            Color32::WHITE,
        );
    }

    changed
}

fn segmented_pill(
    ui: &mut Ui,
    label: &str,
    active: bool,
    active_fill: Color32,
    width: f32,
) -> bool {
    let btn = Button::new(RichText::new(label).size(12.0_f32).color(if active {
        Color32::WHITE
    } else {
        TEXT_SECONDARY
    }))
    .fill(if active {
        active_fill
    } else {
        Color32::TRANSPARENT
    })
    .stroke(Stroke::NONE)
    .rounding(Rounding::same(5.0))
    .min_size(vec2(width, 26.0_f32));

    ui.add(btn).clicked() && !active
}

pub fn render_settings_window(app: &mut PrismApp, ctx: &egui::Context) {
    let mut open = app.show_settings;
    let mut request_close = false;

    // Dimmed backdrop behind the Settings modal
    egui::Area::new(egui::Id::new("settings_backdrop"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::pos2(0.0_f32, 0.0_f32))
        .interactable(true)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            ui.allocate_response(screen.size(), Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0_f32, Color32::from_black_alpha(145));
        });

    Window::new(app.i18n.text("settings-title"))
        .open(&mut open)
        .order(egui::Order::Foreground)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0_f32, 0.0_f32))
        .fixed_size(vec2(520.0_f32, 0.0_f32))
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgb(20, 22, 29))
                .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                .rounding(Rounding::same(12.0))
                .inner_margin(egui::Margin::symmetric(20.0_f32, 16.0_f32)),
        )
        .show(ctx, |ui| {
            ui.set_width(480.0_f32);
            ui.add_space(4.0_f32);

            // Section 1: General
            settings_section_header(ui, &app.i18n.text("settings-section-general"));
            settings_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("settings-language"))
                            .size(13.0_f32)
                            .color(TEXT_PRIMARY),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let is_tr = app.i18n.language() == Language::Turkish;

                        egui::Frame::none()
                            .fill(BG_SURFACE_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .rounding(Rounding::same(7.0))
                            .inner_margin(egui::Margin::same(3.0_f32))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 3.0_f32;
                                if segmented_pill(ui, "Türkçe", is_tr, ACCENT_INDIGO, 82.0_f32) {
                                    app.i18n.set_language(Language::Turkish);
                                    app.config.language = Some("tr".to_string());
                                    let _ = app.config.save();
                                    app.shared_prefs.language = "tr".to_string();
                                    app.shared_prefs.launcher_version = SUITE_VERSION.to_string();
                                    let _ = app.shared_prefs.save();
                                }
                                if segmented_pill(ui, "English", !is_tr, ACCENT_INDIGO, 82.0_f32) {
                                    app.i18n.set_language(Language::English);
                                    app.config.language = Some("en".to_string());
                                    let _ = app.config.save();
                                    app.shared_prefs.language = "en".to_string();
                                    app.shared_prefs.launcher_version = SUITE_VERSION.to_string();
                                    let _ = app.shared_prefs.save();
                                }
                            });
                    });
                });
            });

            ui.add_space(14.0_f32);

            // Section 2: Updates
            settings_section_header(ui, &app.i18n.text("settings-section-updates"));
            settings_card(ui, |ui| {
                // Release channel row
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("settings-channel"))
                            .size(13.0_f32)
                            .color(TEXT_PRIMARY),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let current_channel = app.config.release_channel;

                        egui::Frame::none()
                            .fill(BG_SURFACE_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .rounding(Rounding::same(7.0))
                            .inner_margin(egui::Margin::same(3.0_f32))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 3.0_f32;
                                if segmented_pill(
                                    ui,
                                    &app.i18n.text("settings-channel-beta"),
                                    current_channel == ReleaseChannel::Beta,
                                    ACCENT_AMBER,
                                    72.0_f32,
                                ) {
                                    app.config.release_channel = ReleaseChannel::Beta;
                                    let _ = app.config.save();
                                    app.check_for_updates(true);
                                }
                                if segmented_pill(
                                    ui,
                                    &app.i18n.text("settings-channel-stable"),
                                    current_channel == ReleaseChannel::Stable,
                                    ACCENT_INDIGO,
                                    108.0_f32,
                                ) {
                                    app.config.release_channel = ReleaseChannel::Stable;
                                    let _ = app.config.save();
                                    app.check_for_updates(true);
                                }
                            });
                    });
                });

                ui.add_space(12.0_f32);
                ui.separator();
                ui.add_space(10.0_f32);

                // Check frequency
                ui.label(
                    RichText::new(app.i18n.text("settings-check-frequency"))
                        .size(12.5_f32)
                        .color(TEXT_SECONDARY),
                );
                ui.add_space(8.0_f32);

                let current_freq = app.config.check_frequency;
                let freqs = [
                    (
                        UpdateFrequency::EveryLaunch,
                        app.i18n.text("settings-freq-launch"),
                    ),
                    (UpdateFrequency::Daily, app.i18n.text("settings-freq-daily")),
                    (
                        UpdateFrequency::Weekly,
                        app.i18n.text("settings-freq-weekly"),
                    ),
                    (
                        UpdateFrequency::Manual,
                        app.i18n.text("settings-freq-manual"),
                    ),
                ];

                egui::Frame::none()
                    .fill(BG_SURFACE_ELEVATED)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .rounding(Rounding::same(7.0))
                    .inner_margin(egui::Margin::same(3.0_f32))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0_f32;
                            let pill_w = (ui.available_width() - 9.0_f32) / 4.0_f32;
                            for (freq, label) in freqs {
                                if segmented_pill(
                                    ui,
                                    &label,
                                    current_freq == freq,
                                    ACCENT_INDIGO,
                                    pill_w,
                                ) {
                                    app.config.check_frequency = freq;
                                    let _ = app.config.save();
                                }
                            }
                        });
                    });

                ui.add_space(12.0_f32);
                ui.separator();
                ui.add_space(10.0_f32);

                // Auto-update toggle row
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("settings-auto-update"))
                            .size(13.0_f32)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if render_toggle_switch(ui, &mut app.config.auto_update) {
                            let _ = app.config.save();
                        }
                    });
                });

                ui.add_space(10.0_f32);

                // Check on launch toggle row
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("settings-check-updates-launch"))
                            .size(13.0_f32)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if render_toggle_switch(ui, &mut app.config.check_updates_on_launch) {
                            let _ = app.config.save();
                        }
                    });
                });
            });

            ui.add_space(14.0_f32);

            // Section 3: Storage
            settings_section_header(ui, &app.i18n.text("settings-section-storage"));
            settings_card(ui, |ui| {
                // Keep previous versions stepper
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("settings-keep-versions"))
                            .size(13.0_f32)
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::Frame::none()
                            .fill(BG_SURFACE_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .rounding(Rounding::same(7.0))
                            .inner_margin(egui::Margin::symmetric(4.0_f32, 2.0_f32))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0_f32;
                                let can_inc = app.config.keep_versions_count < 10;
                                if ui
                                    .add_enabled(
                                        can_inc,
                                        Button::new(RichText::new("+").size(13.0_f32).strong())
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE)
                                            .min_size(vec2(24.0_f32, 22.0_f32)),
                                    )
                                    .clicked()
                                {
                                    app.config.keep_versions_count += 1;
                                    let _ = app.config.save();
                                }

                                ui.label(
                                    RichText::new(app.config.keep_versions_count.to_string())
                                        .size(13.0_f32)
                                        .strong()
                                        .color(TEXT_PRIMARY),
                                );

                                let can_dec = app.config.keep_versions_count > 1;
                                if ui
                                    .add_enabled(
                                        can_dec,
                                        Button::new(RichText::new("−").size(13.0_f32).strong())
                                            .fill(Color32::TRANSPARENT)
                                            .stroke(Stroke::NONE)
                                            .min_size(vec2(24.0_f32, 22.0_f32)),
                                    )
                                    .clicked()
                                {
                                    app.config.keep_versions_count -= 1;
                                    let _ = app.config.save();
                                }
                            });
                    });
                });

                ui.add_space(12.0_f32);
                ui.separator();
                ui.add_space(10.0_f32);

                ui.label(
                    RichText::new(app.i18n.text("settings-install-directory"))
                        .size(13.0_f32)
                        .color(TEXT_PRIMARY),
                );
                ui.add_space(4.0_f32);

                let current_dir_str = app
                    .install_mgr
                    .apps_root_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "~/.local/share/prism-suite/apps".to_string());

                ui.label(
                    RichText::new(app.i18n.text_with_arg(
                        "settings-current-dir",
                        "dir",
                        &current_dir_str,
                    ))
                    .size(11.0_f32)
                    .monospace()
                    .color(ACCENT_CYAN),
                );

                ui.add_space(8.0_f32);

                ui.horizontal(|ui| {
                    let input_w = (ui.available_width() - 92.0_f32).max(180.0_f32);
                    ui.add_sized(
                        vec2(input_w, 30.0_f32),
                        egui::TextEdit::singleline(&mut app.custom_dir_input)
                            .hint_text("~/.local/share/prism-suite/apps")
                            .margin(egui::Margin::symmetric(10.0_f32, 6.0_f32)),
                    );

                    ui.add_space(6.0_f32);

                    let save_dir_btn = Button::new(
                        RichText::new(app.i18n.text("action-save"))
                            .size(12.5_f32)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ACCENT_INDIGO)
                    .rounding(Rounding::same(6.0))
                    .min_size(vec2(78.0_f32, 30.0_f32));

                    if ui.add(save_dir_btn).clicked() {
                        let trimmed = app.custom_dir_input.trim();
                        if trimmed.is_empty() {
                            app.config.custom_install_dir = None;
                        } else {
                            app.config.custom_install_dir = Some(PathBuf::from(trimmed));
                        }
                        let _ = app.config.save();
                        app.install_mgr =
                            InstallationManager::new(app.config.custom_install_dir.clone());
                        app.set_temporary_status(app.i18n.text("msg-installed-success"));
                    }
                });
            });

            ui.add_space(16.0_f32);
            ui.separator();
            ui.add_space(12.0_f32);

            ui.horizontal(|ui| {
                let reset_btn = Button::new(
                    RichText::new(app.i18n.text("settings-reset-defaults"))
                        .size(12.0_f32)
                        .color(TEXT_SECONDARY),
                )
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(0.0_f32, 32.0_f32));

                if ui.add(reset_btn).clicked() {
                    app.config.reset_to_defaults();
                    let _ = app.config.save();
                    app.custom_dir_input.clear();
                    app.install_mgr = InstallationManager::new(None);
                    app.check_for_updates(true);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let close_btn = Button::new(
                        RichText::new(app.i18n.text("action-close"))
                            .size(12.5_f32)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ACCENT_INDIGO)
                    .rounding(Rounding::same(6.0))
                    .min_size(vec2(92.0_f32, 32.0_f32));

                    if ui.add(close_btn).clicked() {
                        request_close = true;
                    }
                });
            });
        });

    if request_close {
        open = false;
    }
    app.show_settings = open;
}

pub fn render_about_window(app: &mut PrismApp, ctx: &egui::Context) {
    let about_logo = app.suite_logo_texture(ctx, 64.0);
    let mut open = app.show_about;
    let mut request_close = false;

    Window::new(app.i18n.text("about-title"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .min_width(440.0_f32)
        .max_width(440.0_f32)
        .show(ctx, |ui| {
            ui.set_width(422.0_f32);
            ui.add_space(8.0);

            ui.vertical_centered(|ui| {
                if let Some(tex_id) = about_logo {
                    ui.image((tex_id, vec2(64.0, 64.0)));
                } else {
                    let (logo_rect, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                    draw_suite_logo(ui.painter(), logo_rect);
                }

                ui.add_space(8.0);

                ui.label(
                    RichText::new(SUITE_DISPLAY_NAME)
                        .size(20.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );

                ui.label(
                    RichText::new(app.i18n.text("about-tagline"))
                        .size(12.0)
                        .color(TEXT_MUTED),
                );

                ui.add_space(8.0);

                egui::Frame::none()
                    .fill(ACCENT_CYAN.linear_multiply(0.10))
                    .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.32)))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("v{}  ·  {}", SUITE_VERSION, SUITE_LICENSE))
                                .size(11.5)
                                .strong()
                                .color(ACCENT_CYAN),
                        );
                    });
            });

            ui.add_space(14.0);

            egui::Frame::none()
                .fill(BG_SURFACE)
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(14.0, 12.0))
                .show(ui, |ui| {
                    let arch_val = format!("{} ({})", std::env::consts::ARCH, std::env::consts::OS);
                    let rows = [
                        (app.i18n.text("about-arch"), arch_val),
                        (
                            app.i18n.text("about-graphics"),
                            "egui / glow / OpenGL".to_string(),
                        ),
                        (
                            app.i18n.text("about-developer"),
                            "Murat Özçelik (@MOzcelik14)".to_string(),
                        ),
                        (
                            app.i18n.text("about-ecosystem"),
                            "triM. · steM. · beaM.".to_string(),
                        ),
                    ];

                    for (idx, (label, val)) in rows.iter().enumerate() {
                        if idx > 0 {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);
                        }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(label).size(12.0).color(TEXT_MUTED));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(val).size(12.0).strong().color(TEXT_PRIMARY),
                                );
                            });
                        });
                    }
                });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let half_w = (ui.available_width() - 8.0) / 2.0;

                let repo_btn = Button::new(
                    RichText::new(app.i18n.text("about-launcher-repo"))
                        .size(11.5)
                        .color(ACCENT_CYAN),
                )
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(half_w, 30.0));

                if ui.add(repo_btn).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(LAUNCHER_REPO_URL));
                }

                let org_btn = Button::new(
                    RichText::new(app.i18n.text("about-org-repo"))
                        .size(11.5)
                        .color(ACCENT_CYAN),
                )
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .rounding(Rounding::same(6.0))
                .min_size(vec2(half_w, 30.0));

                if ui.add(org_btn).clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(GITHUB_ORG_URL));
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(app.i18n.text("about-copyright"))
                        .size(11.0)
                        .color(TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let close_btn = Button::new(
                        RichText::new(app.i18n.text("action-close"))
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(ACCENT_INDIGO)
                    .rounding(Rounding::same(5.0))
                    .min_size(vec2(80.0, 28.0));

                    if ui.add(close_btn).clicked() {
                        request_close = true;
                    }
                });
            });
        });

    if request_close {
        open = false;
    }
    app.show_about = open;
}
