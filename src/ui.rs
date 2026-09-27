//! The application window.

use eframe::egui;

use crate::version::VERSION;

/// The application name, used for the window and, later, the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{APP_NAME} {VERSION}: run auto splitters here, control LiveSplit One anywhere")
}

/// The application. Later issues add the status bar and tabs.
pub struct BridgeApp;

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(APP_NAME);
            ui.label(format!("Version {VERSION}"));
            ui.label("Run auto splitters here, control LiveSplit One anywhere.");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VERSION;

    #[test]
    fn title_names_livesplit_one() {
        assert!(window_title().contains("LiveSplit One"));
    }

    #[test]
    fn title_includes_app_name_and_version() {
        let title = window_title();
        assert!(title.starts_with(APP_NAME));
        assert!(title.contains(VERSION));
    }
}
