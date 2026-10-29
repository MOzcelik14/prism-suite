// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{Color32, Rounding, Stroke, Visuals};

pub const BG_PRIMARY: Color32 = Color32::from_rgb(15, 16, 20);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(22, 24, 30);
pub const BG_CARD: Color32 = Color32::from_rgb(26, 28, 36);
pub const BG_CARD_HOVER: Color32 = Color32::from_rgb(34, 37, 48);
pub const BORDER_COLOR: Color32 = Color32::from_rgb(44, 48, 62);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(244, 246, 249);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(152, 160, 176);

pub const ACCENT_INDIGO: Color32 = Color32::from_rgb(99, 102, 241);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(52, 211, 153);
pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(192, 132, 252);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36);

/// Configures egui visuals with a modern dark theme suitable for creative software suites.
pub fn configure_dark_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    visuals.panel_fill = BG_PRIMARY;
    visuals.window_fill = BG_SECONDARY;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_COLOR);
    visuals.window_rounding = Rounding::same(12.0_f32);

    visuals.widgets.noninteractive.bg_fill = BG_CARD;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0_f32);

    visuals.widgets.inactive.bg_fill = BG_SECONDARY;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(8.0_f32);

    visuals.widgets.hovered.bg_fill = BG_CARD_HOVER;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(8.0_f32);

    visuals.widgets.active.bg_fill = ACCENT_INDIGO;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(8.0_f32);

    ctx.set_visuals(visuals);
}
