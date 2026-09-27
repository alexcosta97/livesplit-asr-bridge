mod config;
mod runner;
mod server;
mod ui;
mod version;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(ui::window_title())
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        ui::APP_NAME,
        options,
        Box::new(|cc| {
            ui::install_theme(&cc.egui_ctx);
            Ok(Box::new(ui::BridgeApp::new(&cc.egui_ctx)))
        }),
    )
}
