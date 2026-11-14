// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{Align, Layout, RichText, Rounding, Stroke, Ui};
use egui_commonmark::CommonMarkViewer;

use crate::app::PrismApp;
use crate::theme::{
    ACCENT_INDIGO, BG_SURFACE, BG_SURFACE_ELEVATED, BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY,
};

pub fn render_whats_new_view(app: &mut PrismApp, ui: &mut Ui) {
    let total_width = ui.available_width();
    let max_content_width = 920.0_f32;
    let content_width = (total_width - 48.0).min(max_content_width).max(320.0);
    let side_pad = ((total_width - content_width) * 0.5).max(24.0);

    ui.add_space(22.0);

    ui.horizontal(|ui| {
        ui.add_space(side_pad);
        ui.vertical(|ui| {
            ui.set_width(content_width);

            ui.label(
                RichText::new(app.i18n.text("whats-new-title"))
                    .size(20.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.label(
                RichText::new(app.i18n.text("whats-new-subtitle"))
                    .size(12.0)
                    .color(TEXT_MUTED),
            );

            ui.add_space(18.0);

            if app.discovered_releases.is_empty() {
                egui::Frame::none()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(24.0, 40.0))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(app.i18n.text("whats-new-no-releases"))
                                    .size(13.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                return;
            }

            let releases = app.discovered_releases.clone();
            for (app_name, release) in releases {
                egui::Frame::none()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0))
                    .show(ui, |ui| {
                        ui.set_width(content_width - 40.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&app_name)
                                    .size(16.0)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );

                            ui.add_space(4.0);

                            egui::Frame::none()
                                .fill(ACCENT_INDIGO.linear_multiply(0.16))
                                .stroke(Stroke::new(1.0_f32, ACCENT_INDIGO.linear_multiply(0.35)))
                                .rounding(Rounding::same(5.0))
                                .inner_margin(egui::Margin::symmetric(7.0, 2.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(format!("v{}", release.version))
                                            .size(11.0)
                                            .strong()
                                            .color(ACCENT_INDIGO),
                                    );
                                });

                            if let Some(ref date) = release.published_at {
                                let short_date = date.split('T').next().unwrap_or(date);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    egui::Frame::none()
                                        .fill(BG_SURFACE_ELEVATED)
                                        .rounding(Rounding::same(4.0))
                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(short_date)
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                        });
                                });
                            }
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        if release.release_notes.trim().is_empty() {
                            ui.label(
                                RichText::new(app.i18n.text("details-no-notes"))
                                    .size(12.5)
                                    .color(TEXT_MUTED),
                            );
                        } else {
                            CommonMarkViewer::new().show(
                                ui,
                                &mut app.markdown_cache,
                                &release.release_notes,
                            );
                        }
                    });

                ui.add_space(12.0);
            }
        });
        ui.add_space(side_pad);
    });

    ui.add_space(24.0);
}
