// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;

use chrono::{DateTime, Local, Utc};
use egui::{vec2, Align, Button, Color32, Layout, RichText, Rounding, Stroke, Ui};
use prism_core::catalog::AppId;
use prism_core::state::AppInstallState;

use crate::app::PrismApp;
use crate::theme::{
    ACCENT_INDIGO, ACCENT_RED, BG_SURFACE, BG_SURFACE_ELEVATED, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};

fn format_timestamp(unix_secs: u64) -> String {
    if unix_secs == 0 {
        return String::new();
    }
    let dt = DateTime::<Utc>::from_timestamp(unix_secs as i64, 0).map(|u| u.with_timezone(&Local));
    match dt {
        Some(local) => local.format("%Y-%m-%d %H:%M").to_string(),
        None => String::new(),
    }
}

/// Renders the Recent Projects Hub tab showing recently opened suite projects.
pub fn render_recent_projects_view(app: &mut PrismApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(app.i18n.text("nav-recent-projects"))
                    .size(22.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(app.i18n.text("recent-subtitle"))
                    .size(13.0)
                    .color(TEXT_SECONDARY),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if !app.shared_prefs.recent_projects.is_empty()
                && ui
                    .add(
                        Button::new(
                            RichText::new(app.i18n.text("recent-clear-all"))
                                .size(12.5)
                                .color(TEXT_SECONDARY),
                        )
                        .fill(BG_SURFACE_ELEVATED)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(8.0))
                        .min_size(vec2(110.0, 32.0)),
                    )
                    .clicked()
            {
                app.clear_recent_projects();
            }
        });
    });

    ui.add_space(20.0);

    let projects = app.shared_prefs.recent_projects.clone();
    if projects.is_empty() {
        egui::Frame::none()
            .fill(BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(12.0))
            .inner_margin(egui::Margin::same(28.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("recent-empty-title"))
                            .size(16.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(app.i18n.text("recent-empty-desc"))
                            .size(13.0)
                            .color(TEXT_MUTED),
                    );
                });
            });
        return;
    }

    let mut open_action: Option<(AppId, PathBuf)> = None;
    let mut remove_action: Option<(AppId, PathBuf)> = None;

    for project in &projects {
        let entry = app.catalog.get(project.app_id);
        let app_name = entry
            .map(|e| e.display_name.as_str())
            .unwrap_or(project.app_id.as_str());
        let accent = entry
            .map(|e| Color32::from_rgb(e.accent_color.0, e.accent_color.1, e.accent_color.2))
            .unwrap_or(ACCENT_INDIGO);

        let is_installed = matches!(
            app.states.get(&project.app_id),
            Some(
                AppInstallState::Installed { .. }
                    | AppInstallState::UpdateAvailable { .. }
                    | AppInstallState::Running
            )
        );
        let file_exists = project.path.exists();

        egui::Frame::none()
            .fill(BG_SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .rounding(Rounding::same(12.0))
            .inner_margin(egui::Margin::symmetric(18.0, 14.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    // App color pill
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            36,
                        ))
                        .stroke(Stroke::new(1.0_f32, accent))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(app_name).size(12.0).strong().color(accent));
                        });

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&project.name)
                                    .size(14.5)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            let ts = format_timestamp(project.last_opened_unix);
                            if !ts.is_empty() {
                                ui.add_space(8.0);
                                ui.label(RichText::new(ts).size(11.5).color(TEXT_MUTED));
                            }
                        });

                        ui.label(
                            RichText::new(project.path.display().to_string())
                                .size(11.5)
                                .color(TEXT_SECONDARY),
                        );

                        if !file_exists {
                            ui.label(
                                RichText::new(app.i18n.text("recent-missing-file"))
                                    .size(11.5)
                                    .color(ACCENT_RED),
                            );
                        } else if !is_installed {
                            ui.label(
                                RichText::new(app.i18n.text_with_arg(
                                    "recent-app-not-installed",
                                    "app",
                                    app_name,
                                ))
                                .size(11.5)
                                .color(TEXT_MUTED),
                            );
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                Button::new(
                                    RichText::new(app.i18n.text("recent-remove"))
                                        .size(12.0)
                                        .color(TEXT_SECONDARY),
                                )
                                .fill(BG_SURFACE_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .rounding(Rounding::same(7.0))
                                .min_size(vec2(74.0, 30.0)),
                            )
                            .clicked()
                        {
                            remove_action = Some((project.app_id, project.path.clone()));
                        }

                        ui.add_space(6.0);

                        if ui
                            .add_enabled(
                                is_installed && file_exists,
                                Button::new(
                                    RichText::new(app.i18n.text("recent-open"))
                                        .size(12.5)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(if is_installed && file_exists {
                                    ACCENT_INDIGO
                                } else {
                                    BG_SURFACE_ELEVATED
                                })
                                .rounding(Rounding::same(7.0))
                                .min_size(vec2(74.0, 30.0)),
                            )
                            .clicked()
                        {
                            open_action = Some((project.app_id, project.path.clone()));
                        }
                    });
                });
            });

        ui.add_space(10.0);
    }

    if let Some((app_id, path)) = open_action {
        app.launch_application_with_file(app_id, &path);
    }
    if let Some((app_id, path)) = remove_action {
        app.remove_recent_project(app_id, &path);
    }
}
