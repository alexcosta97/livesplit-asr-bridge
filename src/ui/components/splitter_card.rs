//! Auto splitter card: the loaded file, with Open… and Reload. Wide only;
//! the Splitter strip replaces it when compact.

use eframe::egui::{Label, RichText, Ui};

use super::{
    button::{Button, ButtonSize},
    card::card,
    reload_hint,
    section_label::section_label,
};
use crate::ui::theme;

/// What was clicked on the Auto splitter card or the Splitter strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterAction {
    Open,
    Reload,
    /// Change the game the loaded auto splitter is for.
    Change,
}

/// The Auto splitter card: the loaded file and its game with Change, or "NO
/// AUTO SPLITTER" with help, then Open… and Reload. `game` is the loaded auto
/// splitter's game, if it has one. Open… is disabled while `picking`, when
/// the file picker is already open.
pub fn splitter_card(
    ui: &mut Ui,
    loaded: Option<&str>,
    game: Option<&str>,
    picking: bool,
) -> Option<SplitterAction> {
    card(ui, None, |ui| {
        section_label(ui, "Auto splitter");
        ui.add_space(12.0);
        let mut change = false;
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
                ui.add_space(4.0);
                let (text, color) = match game {
                    Some(game) => (game, theme::TEXT_SECONDARY),
                    None => ("No game", theme::TEXT_MUTED),
                };
                ui.add(Label::new(RichText::new(text).font(theme::body(13.0)).color(color)).wrap());
                ui.add_space(8.0);
                change = ui
                    .add(Button::secondary("Change").size(ButtonSize::Xs))
                    .on_hover_text("Choose which game this auto splitter is for")
                    .clicked();
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
            let open = ui.add(Button::primary("Open…").enabled(!picking));
            let reload = reload_hint(ui.add(Button::secondary("Reload").enabled(loaded.is_some())));
            if change {
                Some(SplitterAction::Change)
            } else if open.clicked() {
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
