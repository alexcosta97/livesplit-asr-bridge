//! The application window.

use std::{collections::VecDeque, sync::Arc, sync::mpsc::Receiver};

use eframe::egui;

use crate::{
    runner::{NoTimer, Runner, RunnerEvent},
    version::VERSION,
};

/// The application name, used for the window and, later, the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The name shown to people, in the window title.
pub const DISPLAY_NAME: &str = "LiveSplit One ASR Bridge";

/// How many recent Runner messages the window keeps, until the Log tab
/// replaces this list.
const RECENT_MESSAGES: usize = 500;

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{DISPLAY_NAME} {VERSION}")
}

/// The application. Later issues add the timer status, connection URL and
/// tabs.
pub struct BridgeApp {
    runner: Runner,
    events: Receiver<RunnerEvent>,
    attached: bool,
    error: Option<String>,
    recent: VecDeque<String>,
}

impl BridgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let ctx = ctx.clone();
        // The Server replaces NoTimer (#10, #11).
        let (runner, events) = Runner::new(Arc::new(NoTimer), move || ctx.request_repaint());
        Self {
            runner,
            events,
            attached: false,
            error: None,
            recent: VecDeque::new(),
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match &event {
                RunnerEvent::Loaded { .. } | RunnerEvent::Unloaded => self.error = None,
                RunnerEvent::GameAttached => self.attached = true,
                RunnerEvent::GameDetached => self.attached = false,
                _ => {}
            }
            let message = event.describe();
            if event.is_error() {
                self.error = Some(message.clone());
            }
            if self.recent.len() == RECENT_MESSAGES {
                self.recent.pop_front();
            }
            self.recent.push_back(message);
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let loaded = self.runner.loaded_path();
        ui.horizontal(|ui| {
            ui.label("Auto splitter:");
            match &loaded {
                Some(path) => ui.strong(crate::runner::file_name(path)),
                None => ui.weak("none"),
            };
            if ui.button("Open…").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Auto splitter", &["wasm"])
                    .pick_file()
            {
                self.runner.load(path);
            }
            if ui
                .add_enabled(loaded.is_some(), egui::Button::new("Reload"))
                .on_hover_text("Load the same file again, for example after rebuilding it")
                .clicked()
            {
                self.runner.reload();
            }
        });
        if loaded.is_some() {
            ui.label(if self.attached {
                "● Attached to the game"
            } else {
                "○ Waiting for game…"
            });
        }
        if let Some(error) = &self.error {
            let mut dismissed = false;
            ui.horizontal(|ui| {
                ui.colored_label(ui.visuals().error_fg_color, error);
                dismissed = ui.small_button("Dismiss").clicked();
            });
            if dismissed {
                self.error = None;
            }
        }
    }
}

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();
        egui::Panel::top("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Recent activity");
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for message in &self.recent {
                        ui.label(message);
                    }
                });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_names_livesplit_one() {
        assert!(window_title().contains("LiveSplit One"));
    }

    #[test]
    fn title_is_display_name_and_version() {
        assert_eq!(
            window_title(),
            format!("LiveSplit One ASR Bridge {VERSION}")
        );
    }
}
