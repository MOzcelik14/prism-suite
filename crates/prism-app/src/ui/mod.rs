// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

pub mod apps_view;
pub mod modals;
pub mod sidebar;
pub mod vector_icons;
pub mod whats_new_view;

use egui::{vec2, Align, Button, Layout, ProgressBar, RichText, Rounding, Stroke, Ui};
use prism_core::branding::{SUITE_LICENSE, SUITE_VERSION};

use crate::app::{ActiveTab, LauncherSelfUpdateState, PrismApp};
use crate::theme::{
    ACCENT_EMERALD, ACCENT_INDIGO, ACCENT_RED, BG_CANVAS, BG_SIDEBAR, BG_SURFACE_ELEVATED,
    BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use vector_icons::{draw_check_icon, draw_info_icon, draw_status_dot};

/// Renders the entire application UI, dispatching sidebar, main view, banners, and modals.
pub fn render_app_ui(app: &mut PrismApp, ctx: &egui::Context) {
    sidebar::render_sidebar(app, ctx);

    // Subtle bottom status bar
    egui::TopBottomPanel::bottom("bottom_status")
        .frame(
            egui::Frame::none()
                .fill(BG_SIDEBAR)
                .inner_margin(egui::Margin::symmetric(24.0_f32, 8.0_f32))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((msg, _)) = &app.status_message {
                    let (dot_rect, _) =
                        ui.allocate_exact_size(vec2(10.0_f32, 10.0_f32), egui::Sense::hover());
                    draw_status_dot(
                        ui.painter(),
                        dot_rect.center(),
                        3.5_f32,
                        ACCENT_EMERALD,
                        true,
                    );
                    ui.add_space(4.0_f32);
                    ui.label(RichText::new(msg).size(11.5_f32).color(TEXT_SECONDARY));
                } else {
                    ui.label(
                        RichText::new(app.i18n.text("suite-subtitle"))
                            .size(11.5_f32)
                            .color(TEXT_MUTED),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("v{}", SUITE_VERSION))
                            .size(11.5_f32)
                            .color(TEXT_MUTED),
                    );
                    ui.label(RichText::new("·").size(11.5_f32).color(TEXT_MUTED));
                    ui.label(
                        RichText::new(SUITE_LICENSE)
                            .size(11.5_f32)
                            .color(TEXT_MUTED),
                    );
                });
            });
        });

    // Main content canvas
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(BG_CANVAS)
                .inner_margin(egui::Margin::same(0.0_f32)),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Frame::none()
                        .inner_margin(egui::Margin::symmetric(28.0_f32, 24.0_f32))
                        .show(ui, |ui| {
                            render_launcher_update_banner(app, ui);

                            match app.active_tab {
                                ActiveTab::Apps => apps_view::render_apps_view(app, ui),
                                ActiveTab::WhatsNew => {
                                    whats_new_view::render_whats_new_view(app, ui)
                                }
                            }
                        });
                });
        });

    // Modal overlays
    if let Some(app_id) = app.selected_detail_app {
        modals::render_details_window(app, ctx, app_id);
    }
    if let Some(app_id) = app.uninstall_target_app {
        modals::render_uninstall_dialog(app, ctx, app_id);
    }
    if app.show_settings {
        modals::render_settings_window(app, ctx);
    }
    if app.show_about {
        modals::render_about_window(app, ctx);
    }
}

fn render_launcher_update_banner(app: &mut PrismApp, ui: &mut Ui) {
    match app.launcher_update_state.clone() {
        LauncherSelfUpdateState::Idle => {}
        LauncherSelfUpdateState::UpdateAvailable(release) => {
            if app.dismissed_launcher_update {
                return;
            }
            egui::Frame::none()
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, ACCENT_INDIGO))
                .rounding(Rounding::same(10.0_f32))
                .inner_margin(egui::Margin::symmetric(16.0_f32, 11.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(vec2(16.0_f32, 16.0_f32), egui::Sense::hover());
                        draw_info_icon(ui.painter(), icon_rect, ACCENT_INDIGO);
                        ui.add_space(6.0_f32);

                        let ver_str = release.version.to_string();
                        let msg =
                            app.i18n
                                .text_with_arg("banner-update-available", "version", &ver_str);
                        ui.label(
                            RichText::new(msg)
                                .size(13.0_f32)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let close_btn =
                                Button::new(RichText::new("x").size(12.0_f32).color(TEXT_MUTED))
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE)
                                    .min_size(vec2(24.0_f32, 24.0_f32));
                            if ui.add(close_btn).clicked() {
                                app.dismissed_launcher_update = true;
                            }

                            ui.add_space(4.0_f32);

                            let update_btn = Button::new(
                                RichText::new(app.i18n.text("action-update-now"))
                                    .size(12.0_f32)
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(ACCENT_INDIGO)
                            .rounding(Rounding::same(7.0_f32))
                            .min_size(vec2(96.0_f32, 28.0_f32));
                            if ui.add(update_btn).clicked() {
                                app.trigger_launcher_self_update(*release);
                            }
                        });
                    });
                });
            ui.add_space(16.0_f32);
        }
        LauncherSelfUpdateState::Updating { progress } => {
            egui::Frame::none()
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, ACCENT_INDIGO))
                .rounding(Rounding::same(10.0_f32))
                .inner_margin(egui::Margin::symmetric(16.0_f32, 11.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(app.i18n.text("status-launcher-updating"))
                                .size(13.0_f32)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(12.0_f32);
                        ui.add(
                            ProgressBar::new(progress)
                                .desired_width(220.0_f32)
                                .desired_height(6.0_f32)
                                .rounding(Rounding::same(3.0_f32))
                                .fill(ACCENT_INDIGO),
                        );
                        ui.label(
                            RichText::new(format!("{}%", (progress * 100.0_f32) as u32))
                                .size(12.0_f32)
                                .color(TEXT_SECONDARY),
                        );
                    });
                });
            ui.add_space(16.0_f32);
        }
        LauncherSelfUpdateState::Updated { version } => {
            egui::Frame::none()
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, ACCENT_EMERALD))
                .rounding(Rounding::same(10.0_f32))
                .inner_margin(egui::Margin::symmetric(16.0_f32, 11.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(vec2(16.0_f32, 16.0_f32), egui::Sense::hover());
                        draw_check_icon(ui.painter(), icon_rect, ACCENT_EMERALD);
                        ui.add_space(6.0_f32);
                        let msg =
                            app.i18n
                                .text_with_arg("msg-launcher-updated", "version", &version);
                        ui.label(
                            RichText::new(msg)
                                .size(13.0_f32)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    });
                });
            ui.add_space(16.0_f32);
        }
        LauncherSelfUpdateState::Error(err) => {
            egui::Frame::none()
                .fill(BG_SURFACE_ELEVATED)
                .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                .rounding(Rounding::same(10.0_f32))
                .inner_margin(egui::Margin::symmetric(16.0_f32, 11.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&err).size(12.5_f32).color(ACCENT_RED));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    Button::new(
                                        RichText::new("x").size(12.0_f32).color(TEXT_MUTED),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .stroke(Stroke::NONE),
                                )
                                .clicked()
                            {
                                app.launcher_update_state = LauncherSelfUpdateState::Idle;
                            }
                        });
                    });
                });
            ui.add_space(16.0_f32);
        }
    }
}
