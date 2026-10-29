mod app;
mod icons;
mod theme;

use app::PrismApp;
use eframe::egui;
use prism_core::branding::SUITE_DISPLAY_NAME;
use theme::configure_dark_theme;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(SUITE_DISPLAY_NAME)
            .with_inner_size([880.0, 620.0])
            .with_min_inner_size([720.0, 480.0])
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
