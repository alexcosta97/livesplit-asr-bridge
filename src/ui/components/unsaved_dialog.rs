//! Unsaved dialog: "Save your settings changes?" when reloading, opening
//! another file, changing the game or closing the app with unsaved settings
//! (spec §6.8).

use eframe::egui::{self, Frame, Id, Label, Margin, Modal, RichText, Stroke, Ui};

use super::{BACKDROP, button::Button};
use crate::ui::theme;

/// The dialog's width.
const WIDTH: f32 = 400.0;

/// What is waiting for the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedTrigger {
    Reload,
    Open,
    ChangeGame,
    Close,
}

/// What was done in the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedAction {
    Save,
    Discard,
    Cancel,
}

/// Shows the Unsaved dialog over the window, for `changes` changed
/// settings. Escape and clicking outside it cancel it.
pub fn unsaved_dialog(
    ctx: &egui::Context,
    trigger: UnsavedTrigger,
    changes: usize,
) -> Option<UnsavedAction> {
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("unsaved dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, trigger, changes));
    if response.should_close() {
        return Some(UnsavedAction::Cancel);
    }
    response.inner
}

fn contents(ui: &mut Ui, trigger: UnsavedTrigger, changes: usize) -> Option<UnsavedAction> {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.add(
        Label::new(
            RichText::new("SAVE YOUR SETTINGS CHANGES?")
                .font(theme::display(16.0))
                .color(theme::TEXT),
        )
        .wrap(),
    );
    ui.add_space(12.0);
    ui.add(
        Label::new(
            RichText::new(message(changes))
                .font(theme::body(14.0))
                .color(theme::TEXT_SECONDARY),
        )
        .wrap(),
    );
    ui.add_space(24.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let save = ui.add(Button::primary(save_label(trigger)));
        let discard = ui.add(Button::secondary("Discard"));
        let cancel = ui.add(Button::secondary("Cancel"));
        if save.clicked() {
            Some(UnsavedAction::Save)
        } else if discard.clicked() {
            Some(UnsavedAction::Discard)
        } else if cancel.clicked() {
            Some(UnsavedAction::Cancel)
        } else {
            None
        }
    })
    .inner
}

/// The text naming how many settings changed.
fn message(changes: usize) -> String {
    match changes {
        1 => "You changed 1 setting and haven't saved it.".to_owned(),
        n => format!("You changed {n} settings and haven't saved them."),
    }
}

/// **Save and reload** for Reload, **Save** otherwise.
fn save_label(trigger: UnsavedTrigger) -> &'static str {
    match trigger {
        UnsavedTrigger::Reload => "Save and reload",
        UnsavedTrigger::Open | UnsavedTrigger::ChangeGame | UnsavedTrigger::Close => "Save",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_how_many_settings_changed() {
        assert_eq!(message(1), "You changed 1 setting and haven't saved it.");
        assert_eq!(message(3), "You changed 3 settings and haven't saved them.");
    }

    #[test]
    fn save_says_it_reloads_only_for_reload() {
        assert_eq!(save_label(UnsavedTrigger::Reload), "Save and reload");
        assert_eq!(save_label(UnsavedTrigger::Open), "Save");
        assert_eq!(save_label(UnsavedTrigger::ChangeGame), "Save");
        assert_eq!(save_label(UnsavedTrigger::Close), "Save");
    }
}
