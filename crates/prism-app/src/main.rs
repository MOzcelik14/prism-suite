// Copyright (C) 2026 Murat Özçelik <mozcelik14>
// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod icons;
mod theme;

use app::PrismApp;
use eframe::egui;
use prism_core::branding::SUITE_DISPLAY_NAME;
use theme::configure_dark_theme;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let icon_data = egui::IconData {
        rgba: include_bytes!("../../../assets/icon.rgba").to_vec(),
        width: 256,
        height: 256,
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
