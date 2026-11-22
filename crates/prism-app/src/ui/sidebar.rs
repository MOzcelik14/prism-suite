// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{vec2, Align, Color32, Layout, RichText, Rounding, Sense, Stroke, Ui};
use prism_core::branding::{LAUNCHER_DISPLAY_NAME, SUITE_LICENSE, SUITE_VERSION};

use crate::app::{ActiveTab, PrismApp};
use crate::icons::draw_suite_logo;
use crate::theme::{
    ACCENT_CYAN, ACCENT_INDIGO, BG_SIDEBAR, BG_SURFACE, BG_SURFACE_ELEVATED, BG_SURFACE_HOVER,
    BORDER_SUBTLE, TEXT_MUTED, TEXT_PRIMARY, TEXT_SECONDARY,
};
use crate::ui::vector_icons::{
    draw_apps_grid_icon, draw_info_icon, draw_notes_icon, draw_recent_icon, draw_refresh_icon,
    draw_settings_icon,
};

pub fn render_sidebar(app: &mut PrismApp, ctx: &egui::Context) {
    let logo_tex = app.suite_logo_texture(ctx, 32.0);
    let (all_cnt, _, upd_cnt) = app.app_counts();

    egui::SidePanel::left("suite_sidebar")
        .resizable(false)
        .exact_width(224.0)
        .frame(
            egui::Frame::none()
                .fill(BG_SIDEBAR)
                .inner_margin(egui::Margin::symmetric(16.0, 18.0))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE)),
        )
        .show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                // Brand Header
                ui.horizontal(|ui| {
                    if let Some(tex_id) = logo_tex {
                        ui.image((tex_id, vec2(32.0, 32.0)));
                    } else {
                        let (logo_rect, _) =
                            ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                        draw_suite_logo(ui.painter(), logo_rect);
                    }

                    ui.add_space(6.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.label(
                                RichText::new(LAUNCHER_DISPLAY_NAME)
                                    .size(17.0)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new("Suite")
                                    .size(17.0)
                                    .strong()
                                    .color(ACCENT_INDIGO),
                            );
                        });
                        ui.label(
                            RichText::new(format!("v{}", SUITE_VERSION))
                                .size(10.5)
                                .color(TEXT_MUTED),
                        );
                    });
                });

                ui.add_space(24.0);

                // Primary Navigation
                let nav_w = ui.available_width();

                if nav_item(
                    ui,
                    nav_w,
                    &app.i18n.text("nav-apps"),
                    Some(all_cnt.to_string()),
                    app.active_tab == ActiveTab::Apps,
                    draw_apps_grid_icon,
                ) {
                    app.active_tab = ActiveTab::Apps;
                }

                ui.add_space(4.0);

                let recent_badge = if !app.shared_prefs.recent_projects.is_empty() {
                    Some(app.shared_prefs.recent_projects.len().to_string())
                } else {
                    None
                };
                if nav_item(
                    ui,
                    nav_w,
                    &app.i18n.text("nav-recent-projects"),
                    recent_badge,
                    app.active_tab == ActiveTab::RecentProjects,
                    draw_recent_icon,
                ) {
                    app.active_tab = ActiveTab::RecentProjects;
                }

                ui.add_space(4.0);

                let notes_badge = if upd_cnt > 0 {
                    Some(upd_cnt.to_string())
                } else {
                    None
                };
                if nav_item(
                    ui,
                    nav_w,
                    &app.i18n.text("nav-whats-new"),
                    notes_badge,
                    app.active_tab == ActiveTab::WhatsNew,
                    draw_notes_icon,
                ) {
                    app.active_tab = ActiveTab::WhatsNew;
                }
            });

            // Bottom Utility Stack
            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                ui.add_space(2.0);

                // Version & license footer inside sidebar
                ui.horizontal(|ui| {
                    ui.label(RichText::new(SUITE_LICENSE).size(10.5).color(TEXT_MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .hyperlink_to(
                                RichText::new(app.i18n.text("footer-github-org"))
                                    .size(10.5)
                                    .color(TEXT_MUTED),
                                "https://github.com/MOzcelik14",
                            )
                            .clicked()
                        {}
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                let btn_w = ui.available_width();

                if utility_button(
                    ui,
                    btn_w,
                    &app.i18n.text("action-about"),
                    true,
                    draw_info_icon,
                ) {
                    app.show_about = true;
                }

                ui.add_space(4.0);

                if utility_button(
                    ui,
                    btn_w,
                    &app.i18n.text("action-settings"),
                    true,
                    draw_settings_icon,
                ) {
                    app.show_settings = true;
                }

                ui.add_space(4.0);

                let refresh_label = if app.is_checking_releases {
                    app.i18n.text("sidebar-checking")
                } else {
                    app.i18n.text("action-refresh")
                };

                if utility_button(
                    ui,
                    btn_w,
                    &refresh_label,
                    !app.is_checking_releases,
                    draw_refresh_icon,
                ) {
                    app.check_for_updates(true);
                    app.set_temporary_status(app.i18n.text("sidebar-checking"));
                }
            });
        });
}

fn nav_item(
    ui: &mut Ui,
    width: f32,
    label: &str,
    badge: Option<String>,
    selected: bool,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> bool {
    let height = 36.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let hovered = resp.hovered();

    let bg = if selected {
        BG_SURFACE_ELEVATED
    } else if hovered {
        BG_SURFACE
    } else {
        Color32::TRANSPARENT
    };

    let border = if selected {
        Stroke::new(1.0_f32, BORDER_SUBTLE)
    } else {
        Stroke::NONE
    };

    ui.painter().rect(rect, Rounding::same(8.0), bg, border);

    // Subtle left indicator bar when selected
    if selected {
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, rect.top() + 8.0),
            vec2(3.0, height - 16.0),
        );
        ui.painter()
            .rect_filled(bar_rect, Rounding::same(1.5), ACCENT_INDIGO);
    }

    let icon_color = if selected {
        TEXT_PRIMARY
    } else if hovered {
        TEXT_SECONDARY
    } else {
        TEXT_MUTED
    };

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        vec2(16.0, 16.0),
    );
    draw_icon(ui.painter(), icon_rect, icon_color);

    let text_color = if selected || hovered {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.0),
        text_color,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 36.0, rect.center().y - galley.size().y * 0.5),
        galley,
        text_color,
    );

    if let Some(badge_text) = badge {
        let badge_galley = ui.painter().layout_no_wrap(
            badge_text,
            egui::FontId::proportional(10.5),
            if selected { ACCENT_CYAN } else { TEXT_MUTED },
        );
        let bw = (badge_galley.size().x + 12.0).max(22.0);
        let bh = 18.0;
        let badge_rect = egui::Rect::from_center_size(
            egui::pos2(rect.right() - 10.0 - bw * 0.5, rect.center().y),
            vec2(bw, bh),
        );
        let badge_bg = if selected {
            ACCENT_INDIGO.linear_multiply(0.22)
        } else {
            BG_SURFACE_HOVER
        };
        ui.painter()
            .rect_filled(badge_rect, Rounding::same(9.0), badge_bg);
        ui.painter().galley(
            egui::pos2(
                badge_rect.center().x - badge_galley.size().x * 0.5,
                badge_rect.center().y - badge_galley.size().y * 0.5,
            ),
            badge_galley,
            TEXT_PRIMARY,
        );
    }

    resp.clicked()
}

fn utility_button(
    ui: &mut Ui,
    width: f32,
    label: &str,
    enabled: bool,
    draw_icon: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> bool {
    let height = 32.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), sense);
    let hovered = enabled && resp.hovered();

    let bg = if hovered {
        BG_SURFACE_ELEVATED
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, Rounding::same(7.0), bg);

    let fg = if !enabled {
        TEXT_MUTED.linear_multiply(0.6)
    } else if hovered {
        TEXT_PRIMARY
    } else {
        TEXT_SECONDARY
    };

    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 16.0, rect.center().y),
        vec2(14.0, 14.0),
    );
    draw_icon(ui.painter(), icon_rect, fg);

    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), egui::FontId::proportional(12.5), fg);
    ui.painter().galley(
        egui::pos2(rect.left() + 32.0, rect.center().y - galley.size().y * 0.5),
        galley,
        fg,
    );

    enabled && resp.clicked()
}
