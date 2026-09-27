//! Error card: the most recent error, at the top of the status column.

use eframe::egui::{RichText, Ui, Vec2};

use super::{
    Width,
    button::{Button, ButtonSize},
    card::{Edge, card},
    status_word::{Dot, status_word},
};
use crate::ui::{status::StatusError, theme};

/// What was clicked on the error card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    Reload,
    Dismiss,
}

/// The error card: the message, Reload after a crash, and Dismiss. Show in
/// log comes with the Log tab (#14).
pub fn error_card(ui: &mut Ui, error: &StatusError, width: Width) -> Option<ErrorAction> {
    let edge = Edge {
        color: theme::STATUS_ERROR,
        tint: theme::TINT_ERROR,
    };
    card(ui, Some(edge), |ui| {
        status_word(ui, Some(Dot::Filled), "Error", theme::STATUS_ERROR, width);
        ui.add_space(12.0);
        let size = match width {
            Width::Wide => 13.0,
            Width::Compact => 14.0,
        };
        ui.label(
            RichText::new(error.message())
                .font(theme::body(size))
                .color(theme::TEXT)
                .line_height(Some((size * 1.45).round())),
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.0);
            let reload = (*error == StatusError::Crashed)
                .then(|| ui.add(Button::secondary("Reload").size(ButtonSize::Sm)));
            let dismiss = ui.add(Button::secondary("Dismiss").size(ButtonSize::Sm));
            if reload.is_some_and(|reload| reload.clicked()) {
                Some(ErrorAction::Reload)
            } else if dismiss.clicked() {
                Some(ErrorAction::Dismiss)
            } else {
                None
            }
        })
        .inner
    })
    .inner
}
