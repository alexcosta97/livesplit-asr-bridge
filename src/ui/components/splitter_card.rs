//! Auto splitter card: the loaded file, with Open… and Reload. Wide only;
//! the Splitter strip replaces it when compact.

use eframe::egui::{Label, RichText, Ui};

use super::{button::Button, card::card, reload_hint, section_label::section_label};
use crate::ui::theme;

/// What was clicked on the Auto splitter card or the Splitter strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterAction {
    Open,
    Reload,
}

/// The Auto splitter card: the loaded file, or "NO AUTO SPLITTER" with help,
/// then Open… and Reload. The game line and Change come with the game
/// association (#7).
pub fn splitter_card(ui: &mut Ui, loaded: Option<&str>) -> Option<SplitterAction> {
    card(ui, None, |ui| {
        section_label(ui, "Auto splitter");
        ui.add_space(12.0);
        match loaded {
            Some(file_name) => {
                ui.add(
                    Label::new(
                        RichText::new(file_name)
                            .font(theme::mono(13.0))
                            .color(theme::TEXT),
                    )
                    .truncate(),
                );
            }
            None => {
                ui.label(
                    RichText::new("NO AUTO SPLITTER")
                        .font(theme::mono(13.0))
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Open a .wasm auto splitter to start.")
                        .font(theme::body(13.0))
                        .color(theme::TEXT_SECONDARY),
                );
            }
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let open = ui.add(Button::primary("Open…"));
            let reload = reload_hint(ui.add(Button::secondary("Reload").enabled(loaded.is_some())));
            if open.clicked() {
                Some(SplitterAction::Open)
            } else if reload.clicked() {
                Some(SplitterAction::Reload)
            } else {
                None
            }
        })
        .inner
    })
    .inner
}
