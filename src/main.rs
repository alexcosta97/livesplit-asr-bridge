mod config;
mod runner;
mod server;
mod ui;
mod version;

use eframe::egui;

fn main() -> eframe::Result {
    // The app ID is the Wayland app ID and the X11 WM class. It matches
    // `StartupWMClass` in the Linux `.desktop` entry, so launchers group the
    // running window with its entry.
    let viewport = egui::ViewportBuilder::default()
        .with_app_id(ui::APP_NAME)
        .with_title(ui::window_title())
        .with_inner_size([800.0, 600.0]);
    let options = eframe::NativeOptions {
        viewport: window_icon(viewport),
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

/// The app icon shown on the window on Linux.
#[cfg(target_os = "linux")]
const ICON_PNG: &[u8] = include_bytes!("../assets/brand/icons/icon-256.png");

/// Sets the window icon on Linux, where X11 shows it on the window (Wayland
/// finds it through the `.desktop` entry). It is not set elsewhere: on macOS
/// the square icon would replace the Dock icon's macOS shape.
#[cfg(target_os = "linux")]
fn window_icon(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    let icon = eframe::icon_data::from_png_bytes(ICON_PNG).expect("the app icon is a valid PNG");
    viewport.with_icon(icon)
}

#[cfg(not(target_os = "linux"))]
fn window_icon(viewport: egui::ViewportBuilder) -> egui::ViewportBuilder {
    viewport
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test]
    fn window_icon_is_a_valid_png() {
        let icon = eframe::icon_data::from_png_bytes(super::ICON_PNG).unwrap();
        assert_eq!((icon.width, icon.height), (256, 256));
    }
}
