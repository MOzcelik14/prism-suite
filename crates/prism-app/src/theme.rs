// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

use egui::{Color32, Rounding, Stroke, Visuals};

// Surface hierarchy
pub const BG_CANVAS: Color32 = Color32::from_rgb(11, 13, 18);
pub const BG_SIDEBAR: Color32 = Color32::from_rgb(15, 18, 25);
pub const BG_SURFACE: Color32 = Color32::from_rgb(20, 24, 34);
pub const BG_SURFACE_ELEVATED: Color32 = Color32::from_rgb(26, 31, 44);
pub const BG_SURFACE_HOVER: Color32 = Color32::from_rgb(33, 39, 55);

// Borders
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(34, 40, 56);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(52, 61, 84);

// Typography
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(241, 244, 249);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(176, 186, 204);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(118, 129, 150);

// Accents
pub const ACCENT_INDIGO: Color32 = Color32::from_rgb(99, 102, 241);
pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const ACCENT_EMERALD: Color32 = Color32::from_rgb(16, 185, 129);
pub const ACCENT_VIOLET: Color32 = Color32::from_rgb(168, 85, 247);
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(217, 119, 6);
pub const ACCENT_RED: Color32 = Color32::from_rgb(220, 38, 38);

/// Configures egui visuals and spacing with a refined dark obsidian desktop theme.
pub fn configure_dark_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(18.0);
    style.spacing.scroll.bar_width = 6.0;
    style.spacing.scroll.bar_inner_margin = 2.0;

    let mut visuals = Visuals::dark();

    visuals.panel_fill = BG_CANVAS;
    visuals.window_fill = BG_SIDEBAR;
    visuals.extreme_bg_color = Color32::from_rgb(8, 10, 14);
    visuals.faint_bg_color = BG_SURFACE;

    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    visuals.window_rounding = Rounding::same(12.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };

    visuals.selection.bg_fill = ACCENT_INDIGO.linear_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_INDIGO);

    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    visuals.widgets.inactive.bg_fill = BG_SURFACE_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(7.0);

    visuals.widgets.hovered.bg_fill = BG_SURFACE_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_STRONG);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(7.0);

    visuals.widgets.active.bg_fill = ACCENT_INDIGO;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT_INDIGO);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(7.0);

    style.visuals = visuals;
    ctx.set_style(style);
}
