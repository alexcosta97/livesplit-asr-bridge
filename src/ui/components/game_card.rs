//! Game card: whether the auto splitter is attached to the game.

use eframe::egui::{Label, RichText, Ui};

use super::{
    Width,
    card::{Edge, card},
    section_label::section_label,
    status_word::{Dot, status_word},
};
use crate::ui::{status::GameState, theme};

/// The Game card: attached, waiting for the game, stopped after a crash, or
/// nothing loaded. `detail` is the line under ATTACHED.
pub fn game_card(ui: &mut Ui, state: &GameState, detail: Option<&str>, width: Width) {
    let edge = (*state == GameState::Attached).then_some(Edge {
        color: theme::STATUS_OK,
        tint: theme::TINT_OK,
    });
    card(ui, edge, |ui| {
        section_label(ui, "Game");
        ui.add_space(8.0);
        let (dot, word, color) = match state {
            GameState::Attached => (Some(Dot::Filled), "Attached", theme::STATUS_OK),
            GameState::Waiting => (
                Some(Dot::Outlined),
                "Waiting for game…",
                theme::STATUS_WAITING,
            ),
            GameState::Stopped => (Some(Dot::Outlined), "Stopped", theme::STATUS_WAITING),
            GameState::None => (None, "—", theme::TEXT_MUTED),
        };
        status_word(ui, dot, word, color, width);
        let help = match state {
            GameState::Attached => None,
            GameState::Waiting => Some("Start the game. This is normal before a run."),
            GameState::Stopped => Some("The auto splitter isn't running"),
            GameState::None => Some("Load an auto splitter first"),
        };
        if let Some(help) = help {
            ui.add_space(4.0);
            ui.label(
                RichText::new(help)
                    .font(theme::body(13.0))
                    .color(theme::TEXT_SECONDARY),
            );
        } else if let Some(detail) = detail {
            ui.add_space(4.0);
            let detail = RichText::new(detail)
                .font(theme::mono(12.0))
                .color(theme::TEXT_SECONDARY);
            ui.add(Label::new(detail).truncate());
        }
    });
}
