// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{
    vec2, Align, Button, Color32, Layout, ProgressBar, RichText, Rounding, Sense, Stroke, Ui,
};
use prism_core::catalog::AppId;
use prism_core::state::AppInstallState;

use crate::app::{AppFilter, PrismApp};
use crate::icons::draw_app_icon;
use crate::theme::{
    ACCENT_AMBER, ACCENT_CYAN, ACCENT_EMERALD, ACCENT_INDIGO, ACCENT_RED, ACCENT_VIOLET,
    BG_SURFACE, BG_SURFACE_ELEVATED, BG_SURFACE_HOVER, BORDER_STRONG, BORDER_SUBTLE, TEXT_MUTED,
    TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::vector_icons::{draw_external_arrow, draw_status_dot};

pub fn render_apps_view(app: &mut PrismApp, ui: &mut Ui) {
    let total_width = ui.available_width();
    let max_content_width = 1120.0_f32;
    let content_width = (total_width - 48.0).min(max_content_width).max(320.0);
    let side_pad = ((total_width - content_width) * 0.5).max(24.0);

    ui.add_space(22.0);

    ui.horizontal(|ui| {
        ui.add_space(side_pad);
        ui.vertical(|ui| {
            ui.set_width(content_width);

            let (all_cnt, inst_cnt, upd_cnt) = app.app_counts();

            // Header & Filter Bar
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(app.i18n.text("section-creative-apps"))
                            .size(20.0)
                            .strong()
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(app.i18n.text_with_arg(
                            "apps-count-label",
                            "count",
                            &all_cnt.to_string(),
                        ))
                        .size(12.0)
                        .color(TEXT_MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if upd_cnt > 0 {
                        let update_all_btn = Button::new(
                            RichText::new(format!(
                                "{} ({})",
                                app.i18n.text("btn-update-all"),
                                upd_cnt
                            ))
                            .strong()
                            .size(12.0)
                            .color(Color32::WHITE),
                        )
                        .fill(ACCENT_INDIGO)
                        .rounding(Rounding::same(7.0))
                        .min_size(vec2(0.0, 32.0));

                        if ui.add(update_all_btn).clicked() {
                            app.update_all_apps();
                        }
                        ui.add_space(10.0);
                    }

                    // Segmented Filter Container
                    egui::Frame::none()
                        .fill(BG_SURFACE)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(egui::Margin::same(3.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let filters = [
                                    (
                                        AppFilter::Updates,
                                        format!(
                                            "{} ({})",
                                            app.i18n.text("filter-updates"),
                                            upd_cnt
                                        ),
                                    ),
                                    (
                                        AppFilter::Installed,
                                        format!(
                                            "{} ({})",
                                            app.i18n.text("filter-installed"),
                                            inst_cnt
                                        ),
                                    ),
                                    (
                                        AppFilter::All,
                                        format!("{} ({})", app.i18n.text("filter-all"), all_cnt),
                                    ),
                                ];
                                for (f, label) in filters.into_iter().rev() {
                                    let active = app.app_filter == f;
                                    let btn = Button::new(
                                        RichText::new(label).size(11.5).color(if active {
                                            TEXT_PRIMARY
                                        } else {
                                            TEXT_MUTED
                                        }),
                                    )
                                    .fill(if active {
                                        BG_SURFACE_ELEVATED
                                    } else {
                                        Color32::TRANSPARENT
                                    })
                                    .stroke(if active {
                                        Stroke::new(1.0_f32, BORDER_STRONG)
                                    } else {
                                        Stroke::NONE
                                    })
                                    .rounding(Rounding::same(6.0))
                                    .min_size(vec2(0.0, 26.0));

                                    if ui.add(btn).clicked() {
                                        app.app_filter = f;
                                    }
                                }
                            });
                        });
                });
            });

            ui.add_space(18.0);

            // Responsive Grid
            let filtered_apps = app.filtered_apps();
            if filtered_apps.is_empty() {
                egui::Frame::none()
                    .fill(BG_SURFACE)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::symmetric(24.0, 48.0))
                    .show(ui, |ui| {
                        ui.set_width(content_width);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(app.i18n.text("filter-no-results"))
                                    .size(13.5)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
            } else if content_width >= 580.0 {
                let cols = 2;
                let spacing = 16.0_f32;
                let card_w = (content_width - spacing) / 2.0_f32;

                ui.columns(cols, |columns| {
                    for (i, entry) in filtered_apps.iter().enumerate() {
                        let col_idx = i % cols;
                        render_app_card(app, &mut columns[col_idx], entry, card_w);
                        columns[col_idx].add_space(16.0);
                    }
                });
            } else {
                for entry in &filtered_apps {
                    render_app_card(app, ui, entry, content_width);
                    ui.add_space(16.0);
                }
            }

            ui.add_space(22.0);

            // Subtle Suite Note
            egui::Frame::none()
                .fill(BG_SURFACE.linear_multiply(0.75))
                .rounding(Rounding::same(8.0))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(16.0, 12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("prisM. Suite")
                                .size(12.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(RichText::new("·").size(12.0).color(TEXT_MUTED));
                        ui.label(
                            RichText::new(app.i18n.text("footer-open-source-note"))
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let status_label = if upd_cnt > 0 {
                                format!(
                                    "{} ({})",
                                    app.i18n.text("status-update-available"),
                                    upd_cnt
                                )
                            } else {
                                app.i18n.text("home-all-up-to-date")
                            };
                            ui.label(RichText::new(status_label).size(11.5).color(TEXT_MUTED));
                        });
                    });
                });
        });
        ui.add_space(side_pad);
    });

    ui.add_space(24.0);
}

fn render_app_card(app: &mut PrismApp, ui: &mut Ui, entry: &prism_core::AppEntry, card_width: f32) {
    let current_state =
        app.states
            .get(&entry.id)
            .cloned()
            .unwrap_or(AppInstallState::NotInstalled {
                latest_release: None,
            });

    let mut clicked_details = false;
    let card_height = 232.0_f32;

    let accent_col = match entry.id {
        AppId::Trim => ACCENT_CYAN,
        AppId::Stem => ACCENT_EMERALD,
        AppId::Beam => ACCENT_VIOLET,
        AppId::Dram => ACCENT_AMBER,
    };

    let card_resp = ui.allocate_ui(vec2(card_width, card_height), |ui| {
        ui.set_min_size(vec2(card_width, card_height));
        ui.set_max_size(vec2(card_width, card_height));

        egui::Frame::none()
            .fill(BG_SURFACE)
            .rounding(Rounding::same(10.0))
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .inner_margin(egui::Margin::symmetric(16.0, 14.0))
            .show(ui, |ui| {
                ui.set_min_size(vec2(card_width - 32.0, card_height - 28.0));
                ui.set_max_size(vec2(card_width - 32.0, card_height - 28.0));

                // Bottom-up layout locks action controls to the exact same baseline across cards
                ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                    if let AppInstallState::Downloading { progress, .. } = current_state {
                        ui.horizontal(|ui| {
                            let cancel_w = 78.0;
                            let bar_w = (ui.available_width() - cancel_w - 8.0).max(60.0);

                            ui.vertical(|ui| {
                                ui.add_space(4.0);
                                ui.add(
                                    ProgressBar::new(progress)
                                        .desired_width(bar_w)
                                        .text(format!("{:.0}%", progress * 100.0)),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let cancel_btn = Button::new(
                                    RichText::new(app.i18n.text("action-cancel-download"))
                                        .size(11.5)
                                        .color(TEXT_SECONDARY),
                                )
                                .fill(BG_SURFACE_ELEVATED)
                                .stroke(Stroke::new(1.0_f32, BORDER_STRONG))
                                .rounding(Rounding::same(7.0))
                                .min_size(vec2(cancel_w, 34.0));

                                if ui.add(cancel_btn).clicked() {
                                    app.cancel_download(entry.id);
                                }
                            });
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let details_w = 82.0_f32;
                            let primary_w = (ui.available_width() - details_w - 8.0).max(90.0);

                            render_primary_action(app, ui, entry, &current_state, primary_w);

                            let details_btn = Button::new(
                                RichText::new(app.i18n.text("action-details"))
                                    .size(12.0)
                                    .color(TEXT_SECONDARY),
                            )
                            .fill(BG_SURFACE_ELEVATED)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .rounding(Rounding::same(7.0))
                            .min_size(vec2(details_w, 34.0));

                            if ui.add(details_btn).clicked() {
                                clicked_details = true;
                            }
                        });
                    }

                    ui.add_space(10.0);

                    // Main Card Body
                    ui.with_layout(Layout::top_down(Align::Min), |ui| {
                        // Top Row: Icon + Title + Category + Subtitle + External Link Button
                        ui.horizontal(|ui| {
                            let icon_size = vec2(44.0, 44.0);
                            if let Some(tex) = app.app_textures.get(&entry.id) {
                                ui.add(
                                    egui::Image::new((tex.id(), icon_size))
                                        .rounding(Rounding::same(9.0)),
                                );
                            } else {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(icon_size, Sense::hover());
                                draw_app_icon(ui.painter(), icon_rect, entry.id);
                            }

                            ui.add_space(10.0);

                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&entry.display_name)
                                            .size(17.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );

                                    let cat_name = match entry.id {
                                        AppId::Trim => app.i18n.text("category-trim"),
                                        AppId::Stem => app.i18n.text("category-stem"),
                                        AppId::Beam => app.i18n.text("category-beam"),
                                        AppId::Dram => app.i18n.text("category-dram"),
                                    };
                                    egui::Frame::none()
                                        .fill(accent_col.linear_multiply(0.14))
                                        .stroke(Stroke::new(
                                            1.0_f32,
                                            accent_col.linear_multiply(0.32),
                                        ))
                                        .rounding(Rounding::same(4.0))
                                        .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(cat_name)
                                                    .size(9.5)
                                                    .strong()
                                                    .color(accent_col),
                                            );
                                        });
                                });

                                let comparison = match entry.id {
                                    AppId::Trim => app.i18n.text("compare-trim"),
                                    AppId::Stem => app.i18n.text("compare-stem"),
                                    AppId::Beam => app.i18n.text("compare-beam"),
                                    AppId::Dram => app.i18n.text("compare-dram"),
                                };
                                ui.label(RichText::new(comparison).size(11.0).color(TEXT_MUTED));
                            });

                            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                                let (ext_rect, ext_resp) =
                                    ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                                let ext_bg = if ext_resp.hovered() {
                                    BG_SURFACE_HOVER
                                } else {
                                    BG_SURFACE_ELEVATED
                                };
                                ui.painter().rect(
                                    ext_rect,
                                    Rounding::same(6.0),
                                    ext_bg,
                                    Stroke::new(1.0_f32, BORDER_SUBTLE),
                                );
                                draw_external_arrow(
                                    ui.painter(),
                                    ext_rect.shrink(5.0),
                                    if ext_resp.hovered() {
                                        TEXT_PRIMARY
                                    } else {
                                        TEXT_MUTED
                                    },
                                );
                                if ext_resp
                                    .on_hover_text(app.i18n.text_with_arg(
                                        "tooltip-open-github",
                                        "app",
                                        &entry.display_name,
                                    ))
                                    .clicked()
                                {
                                    ui.ctx().open_url(egui::OpenUrl::new_tab(&entry.repository));
                                }
                            });
                        });

                        ui.add_space(10.0);

                        // Status Capsule
                        render_card_status_capsule(app, ui, &current_state);

                        ui.add_space(8.0);

                        // Tagline (fixed height for strict grid alignment)
                        let avail_w = ui.available_width();
                        ui.allocate_ui_with_layout(
                            vec2(avail_w, 32.0),
                            Layout::top_down(Align::Min),
                            |ui| {
                                ui.label(
                                    RichText::new(app.i18n.text(&entry.tagline_key))
                                        .size(11.5)
                                        .color(TEXT_SECONDARY),
                                );
                            },
                        );

                        ui.add_space(6.0);

                        // Technical capability chips
                        ui.horizontal(|ui| {
                            let lang = app.i18n.language().as_code();
                            let chips: &[&str] = match (entry.id, lang) {
                                (AppId::Trim, "tr") => &["FFmpeg 7", "HW Hızlandırma", "Doğal NLE"],
                                (AppId::Trim, _) => &["FFmpeg 7", "HW Acceleration", "Native NLE"],
                                (AppId::Stem, "tr") => {
                                    &["PipeWire / ALSA", "32-bit DAW", "Düşük Gecikme"]
                                }
                                (AppId::Stem, _) => {
                                    &["PipeWire / ALSA", "32-bit DAW", "Low Latency"]
                                }
                                (AppId::Beam, "tr") => &["wgpu Vulkan", "Rec.2020", "RAW İşleme"],
                                (AppId::Beam, _) => &["wgpu Vulkan", "Rec.2020", "RAW Engine"],
                                (AppId::Dram, "tr") => &["Courier Prime", "PDF / FDX", "TR / EN"],
                                (AppId::Dram, _) => &["Courier Prime", "PDF / FDX", "TR / EN"],
                            };

                            for &chip in chips {
                                egui::Frame::none()
                                    .fill(BG_SURFACE_ELEVATED)
                                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                    .rounding(Rounding::same(4.0))
                                    .inner_margin(egui::Margin::symmetric(6.0, 2.5))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(chip).size(9.5).color(TEXT_MUTED));
                                    });
                            }
                        });
                    });
                });
            });
    });

    let card_rect = card_resp.response.rect;
    let is_hovered = ui.rect_contains_pointer(card_rect);

    // Subtle top accent hairline
    let top_bar = egui::Rect::from_min_size(card_rect.min, vec2(card_rect.width(), 2.0));
    let top_col = if is_hovered {
        accent_col
    } else {
        accent_col.linear_multiply(0.38)
    };
    ui.painter().rect_filled(
        top_bar,
        Rounding {
            nw: 10.0,
            ne: 10.0,
            sw: 0.0,
            se: 0.0,
        },
        top_col,
    );

    if is_hovered {
        ui.painter().rect_stroke(
            card_rect,
            Rounding::same(10.0),
            Stroke::new(1.0_f32, accent_col.linear_multiply(0.55)),
        );
    }

    if clicked_details {
        app.selected_detail_app = Some(entry.id);
    }
}

pub fn render_card_status_capsule(app: &PrismApp, ui: &mut Ui, state: &AppInstallState) {
    let (label, color, filled) = match state {
        AppInstallState::NotInstalled { latest_release } => {
            if let Some(r) = latest_release {
                (
                    format!(
                        "v{} · {}",
                        r.version,
                        app.i18n.text("status-available-to-install")
                    ),
                    ACCENT_CYAN,
                    false,
                )
            } else {
                (app.i18n.text("status-in-development"), TEXT_MUTED, false)
            }
        }
        AppInstallState::NoReleaseAvailable => {
            (app.i18n.text("status-in-development"), TEXT_MUTED, false)
        }
        AppInstallState::Installed { version, .. } => (
            format!("v{} · {}", version, app.i18n.text("status-up-to-date")),
            ACCENT_EMERALD,
            true,
        ),
        AppInstallState::UpdateAvailable {
            current_version,
            latest_release,
            ..
        } => (
            format!(
                "v{} -> v{} · {}",
                current_version,
                latest_release.version,
                app.i18n.text("status-update-available")
            ),
            ACCENT_AMBER,
            true,
        ),
        AppInstallState::Downloading { progress, .. } => (
            format!(
                "{} {:.0}%",
                app.i18n.text("status-downloading"),
                progress * 100.0
            ),
            ACCENT_CYAN,
            true,
        ),
        AppInstallState::Verifying => (app.i18n.text("status-verifying"), ACCENT_VIOLET, true),
        AppInstallState::Installing => (app.i18n.text("status-installing"), ACCENT_VIOLET, true),
        AppInstallState::Running => (app.i18n.text("status-running"), ACCENT_EMERALD, true),
        AppInstallState::Error { message } => (message.clone(), ACCENT_RED, true),
    };

    egui::Frame::none()
        .fill(color.linear_multiply(0.10))
        .stroke(Stroke::new(1.0_f32, color.linear_multiply(0.32)))
        .rounding(Rounding::same(5.0))
        .inner_margin(egui::Margin::symmetric(7.0, 3.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let (dot_rect, _) = ui.allocate_exact_size(vec2(8.0, 12.0), Sense::hover());
                draw_status_dot(ui.painter(), dot_rect.center(), 3.2, color, filled);
                ui.label(RichText::new(label).size(10.5).strong().color(color));
            });
        });
}

fn render_primary_action(
    app: &mut PrismApp,
    ui: &mut Ui,
    entry: &prism_core::AppEntry,
    state: &AppInstallState,
    btn_width: f32,
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
                .rounding(Rounding::same(7.0))
                .min_size(vec2(btn_width, 34.0));

                if ui.add(btn).clicked() {
                    app.start_install_pipeline(entry.id, rel.clone());
                }
            } else {
                let btn = Button::new(
                    RichText::new("GitHub")
                        .size(12.0)
                        .strong()
                        .color(ACCENT_CYAN),
                )
                .fill(ACCENT_CYAN.linear_multiply(0.10))
                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.35)))
                .rounding(Rounding::same(7.0))
                .min_size(vec2(btn_width, 34.0));

                if ui
                    .add(btn)
                    .on_hover_text(app.i18n.text_with_arg(
                        "tooltip-open-github",
                        "app",
                        &entry.display_name,
                    ))
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&entry.repository));
                }
            }
        }
        AppInstallState::NoReleaseAvailable => {
            let btn = Button::new(
                RichText::new("GitHub")
                    .size(12.0)
                    .strong()
                    .color(ACCENT_CYAN),
            )
            .fill(ACCENT_CYAN.linear_multiply(0.10))
            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN.linear_multiply(0.35)))
            .rounding(Rounding::same(7.0))
            .min_size(vec2(btn_width, 34.0));

            if ui
                .add(btn)
                .on_hover_text(app.i18n.text_with_arg(
                    "tooltip-open-github",
                    "app",
                    &entry.display_name,
                ))
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&entry.repository));
            }
        }
        AppInstallState::Installed { .. } => {
            let btn = Button::new(
                RichText::new(app.i18n.text("action-launch"))
                    .size(12.5)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT_EMERALD)
            .rounding(Rounding::same(7.0))
            .min_size(vec2(btn_width, 34.0));

            if ui.add(btn).clicked() {
                app.launch_application(entry.id);
            }
        }
        AppInstallState::UpdateAvailable { latest_release, .. } => {
            let btn = Button::new(
                RichText::new(app.i18n.text("action-update"))
                    .size(12.5)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT_AMBER)
            .rounding(Rounding::same(7.0))
            .min_size(vec2(btn_width, 34.0));

            let rel_clone = latest_release.clone();
            if ui.add(btn).clicked() {
                app.start_install_pipeline(entry.id, rel_clone);
            }
        }
        AppInstallState::Running => {
            let btn = Button::new(
                RichText::new(app.i18n.text("status-running"))
                    .size(12.0)
                    .strong()
                    .color(ACCENT_EMERALD),
            )
            .fill(ACCENT_EMERALD.linear_multiply(0.12))
            .stroke(Stroke::new(1.0_f32, ACCENT_EMERALD.linear_multiply(0.40)))
            .rounding(Rounding::same(7.0))
            .min_size(vec2(btn_width, 34.0));

            ui.add_enabled(false, btn);
        }
        AppInstallState::Error { .. } => {
            let btn = Button::new(
                RichText::new(app.i18n.text("action-retry"))
                    .size(12.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(ACCENT_INDIGO)
            .rounding(Rounding::same(7.0))
            .min_size(vec2(btn_width, 34.0));

            if ui.add(btn).clicked() {
                app.check_for_updates(true);
            }
        }
        AppInstallState::Verifying | AppInstallState::Installing => {
            let label = match state {
                AppInstallState::Verifying => app.i18n.text("status-verifying"),
                _ => app.i18n.text("status-installing"),
            };
            let btn = Button::new(RichText::new(label).size(12.0).color(ACCENT_VIOLET))
                .fill(ACCENT_VIOLET.linear_multiply(0.10))
                .stroke(Stroke::new(1.0_f32, ACCENT_VIOLET.linear_multiply(0.35)))
                .rounding(Rounding::same(7.0))
                .min_size(vec2(btn_width, 34.0));

            ui.add_enabled(false, btn);
        }
        AppInstallState::Downloading { .. } => {}
    }
}
