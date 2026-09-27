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
    ShowInLog,
    Reload,
    OpenConnection,
    Dismiss,
}

/// The error card: the message, Show in log for the errors that need more
/// detail, Reload after a crash, Open Connection after a server error, and
/// Dismiss.
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
            let show_in_log = error
                .log_category()
                .map(|_| ui.add(Button::secondary("Show in log").size(ButtonSize::Sm)));
            let reload = (*error == StatusError::Crashed)
                .then(|| ui.add(Button::secondary("Reload").size(ButtonSize::Sm)));
            let open_connection = error
                .is_server()
                .then(|| ui.add(Button::secondary("Open Connection").size(ButtonSize::Sm)));
            let dismiss = ui.add(Button::secondary("Dismiss").size(ButtonSize::Sm));
            if show_in_log.is_some_and(|show| show.clicked()) {
                Some(ErrorAction::ShowInLog)
            } else if reload.is_some_and(|reload| reload.clicked()) {
                Some(ErrorAction::Reload)
            } else if open_connection.is_some_and(|open| open.clicked()) {
                Some(ErrorAction::OpenConnection)
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

#[cfg(test)]
mod tests {
    use eframe::egui::{CentralPanel, Context, RawInput};

    use super::*;

    /// The labels of the buttons on the card for `error`.
    fn buttons(error: &StatusError, width: Width) -> Vec<String> {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| error_card(ui, error, width));
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        let mut labels: Vec<_> = output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, node)| node.role() == eframe::egui::accesskit::Role::Button)
            .filter_map(|(_, node)| node.label().map(str::to_owned))
            .collect();
        labels.sort();
        labels
    }

    #[test]
    fn each_kind_has_its_buttons_wide_and_compact() {
        let cases = [
            (
                StatusError::Crashed,
                vec!["Dismiss", "Reload", "Show in log"],
            ),
            (
                StatusError::LoadFailed {
                    message: "Couldn't load foo.wasm".to_owned(),
                },
                vec!["Dismiss", "Show in log"],
            ),
            (
                StatusError::PortInUse { port: 16834 },
                vec!["Dismiss", "Open Connection"],
            ),
        ];
        for (error, expected) in cases {
            for width in [Width::Wide, Width::Compact] {
                assert_eq!(buttons(&error, width), expected, "{error:?} {width:?}");
            }
        }
    }
}
