// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod icons;
mod logo;
mod theme;

use app::PrismApp;
use eframe::egui;
use prism_core::branding::SUITE_DISPLAY_NAME;
use theme::configure_dark_theme;

fn main() -> eframe::Result<()> {
    env_logger::init();

    // Window icon is rasterized from the vector logo; falls back to the bundled bitmap.
    let icon_data = match logo::render_logo_rgba(256) {
        Some(premul) => egui::IconData {
            rgba: unpremultiply(&premul),
            width: 256,
            height: 256,
        },
        None => egui::IconData {
            rgba: include_bytes!("../../../assets/icon.rgba").to_vec(),
            width: 256,
            height: 256,
        },
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(SUITE_DISPLAY_NAME)
            .with_icon(icon_data)
            .with_inner_size([1160.0, 760.0])
            .with_min_inner_size([880.0, 580.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        SUITE_DISPLAY_NAME,
        native_options,
        Box::new(|cc| {
            configure_dark_theme(&cc.egui_ctx);
            configure_fonts(&cc.egui_ctx);
            Ok(Box::new(PrismApp::new(cc)))
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 1. Embed DejaVu Sans for complete Turkish glyph coverage (ç ğ ı İ ö ş ü Ç Ğ Ö Ş Ü)
    //    and Unicode symbols (✓, ●, —, •, ↗, ↩, ⚙, ⬆, etc.).
    const DEJAVU_SANS: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
    fonts.font_data.insert(
        "dejavu_sans".to_owned(),
        egui::FontData::from_static(DEJAVU_SANS),
    );

    // 2. Scan and load any available system symbol / fallback fonts
    let candidate_system_fonts = [
        "/usr/share/fonts/truetype/noto/NotoSansSymbols-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    for (idx, path) in candidate_system_fonts.iter().enumerate() {
        if let Ok(data) = std::fs::read(path) {
            let name = format!("sys_font_{}", idx);
            fonts
                .font_data
                .insert(name.clone(), egui::FontData::from_owned(data));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(name.clone());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(name);
        }
    }

    // 3. Make DejaVu Sans the primary proportional font
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "dejavu_sans".to_owned());

    // 4. Ensure Hack is also in proportional fallback chain so monospace symbols (e.g. ●) are available
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("Hack".to_owned());

    // 5. Add DejaVu Sans to monospace fallback as well
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("dejavu_sans".to_owned());

    ctx.set_fonts(fonts);
}

fn unpremultiply(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for i in (0..out.len().saturating_sub(3)).step_by(4) {
        let a = out[i + 3] as u32;
        if a > 0 && a < 255 {
            for c in &mut out[i..i + 3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    out
}
