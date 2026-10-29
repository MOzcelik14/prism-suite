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
    let fonts = egui::FontDefinitions::default();

    // Default proportional fonts in egui 0.29 have complete Latin Extended-A coverage.
    // If needed, custom font families can be prepended here.
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
