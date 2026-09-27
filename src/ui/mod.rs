//! The application window: the status column and, when wide, the area the
//! tabs will fill.

mod components;
mod status;
mod theme;

use std::{collections::VecDeque, sync::Arc, sync::mpsc::Receiver};

use eframe::egui::{self, Frame, Label, Margin, RichText, ScrollArea};

use crate::{
    runner::{NoTimer, Runner, RunnerEvent, file_name},
    version::VERSION,
};
use components::{ErrorAction, SplitterAction, Width};
use status::Status;

pub use theme::install as install_theme;

/// The application name, used for the window and, later, the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The name shown to people, in the window title.
pub const DISPLAY_NAME: &str = "LiveSplit One ASR Bridge";

/// How many recent Runner messages the window keeps, until the Log tab
/// replaces this list.
const RECENT_MESSAGES: usize = 500;

/// Below this window width, the status column fills the window.
const COMPACT_BELOW: f32 = 640.0;
/// The width of the status column, when wide.
const COLUMN_WIDTH: f32 = 300.0;
/// The space between cards in the status column.
const CARD_GAP: f32 = 12.0;

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{DISPLAY_NAME} {VERSION}")
}

/// The application. Later issues add the Timer and Last action cards and the
/// tabs.
pub struct BridgeApp {
    runner: Runner,
    events: Receiver<RunnerEvent>,
    status: Status,
    /// Each message, and whether it is an error.
    recent: VecDeque<(String, bool)>,
}

impl BridgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let ctx = ctx.clone();
        // The Server replaces NoTimer (#10, #11).
        let (runner, events) = Runner::new(Arc::new(NoTimer), move || ctx.request_repaint());
        Self {
            runner,
            events,
            status: Status::default(),
            recent: VecDeque::new(),
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.status.apply(&event);
            if self.recent.len() == RECENT_MESSAGES {
                self.recent.pop_front();
            }
            self.recent.push_back((event.describe(), event.is_error()));
        }
    }

    /// The status column: the header, the error card while there is one, the
    /// auto splitter and the game. It scrolls rather than clip a card.
    fn status_column(&mut self, ui: &mut egui::Ui, width: Width) {
        let loaded = self.runner.loaded_path().map(|path| file_name(&path));
        let loaded = loaded.as_deref();
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            components::app_header(ui, width);

            if let Some(error) = self.status.error() {
                match components::error_card(ui, error, width) {
                    Some(ErrorAction::Reload) => self.runner.reload(),
                    Some(ErrorAction::Dismiss) => self.status.dismiss_error(),
                    None => {}
                }
            }

            let action = match width {
                Width::Wide => components::splitter_card(ui, loaded),
                Width::Compact => components::splitter_strip(ui, loaded),
            };
            match action {
                Some(SplitterAction::Open) => self.open(),
                Some(SplitterAction::Reload) => self.runner.reload(),
                None => {}
            }

            let detail = self.status.game_detail(width == Width::Compact);
            components::game_card(ui, &self.status.game(), detail.as_deref(), width);
        });
    }

    /// Asks for a `.wasm` file and loads it.
    fn open(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Auto splitter", &["wasm"])
            .pick_file()
        {
            self.runner.load(path);
        }
    }

    /// The Runner's recent messages, until the Log tab (#12) replaces them.
    fn recent_activity(&self, ui: &mut egui::Ui) {
        components::section_label(ui, "Recent activity");
        ui.add_space(12.0);
        ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for (message, is_error) in &self.recent {
                    let color = if *is_error {
                        theme::STATUS_ERROR
                    } else {
                        theme::TEXT
                    };
                    ui.add(
                        Label::new(RichText::new(message).font(theme::mono(12.0)).color(color))
                            .wrap(),
                    );
                }
            });
    }
}

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();
        let column = Frame::NONE
            .fill(theme::PANEL)
            .inner_margin(Margin::same(16));
        if ui.max_rect().width() < COMPACT_BELOW {
            egui::CentralPanel::default()
                .frame(column)
                .show(ui, |ui| self.status_column(ui, Width::Compact));
            return;
        }
        egui::Panel::left("status")
            .exact_size(COLUMN_WIDTH)
            .resizable(false)
            .frame(column)
            .show(ui, |ui| self.status_column(ui, Width::Wide));
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(theme::WINDOW)
                    .inner_margin(Margin::same(24)),
            )
            .show(ui, |ui| self.recent_activity(ui));
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
