//! Unsaved dialog: "Save your settings changes?" when reloading, opening
//! another file, changing the game or the category, or closing the app with
//! unsaved settings (spec §6.8).

use eframe::egui::{
    self, Align, Frame, Id, Label, Layout, Margin, Modal, RichText, Stroke, Ui, vec2,
};

use super::{BACKDROP, button::Button};
use crate::ui::theme;

/// The width inside the padding: the dialog is 440 px wide.
const WIDTH: f32 = 392.0;

/// The buttons row's height: the height of a button.
const BUTTONS_HEIGHT: f32 = 32.0;

/// What is waiting for the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsavedTrigger {
    Reload,
    Open,
    ChangeGame,
    /// Switching to another category in the Category dialog.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used by the Category dialog, added next")
    )]
    ChangeCategory,
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
/// settings of `game`. Escape and clicking outside it cancel it.
pub fn unsaved_dialog(
    ctx: &egui::Context,
    trigger: UnsavedTrigger,
    changes: usize,
    game: Option<&str>,
    category: Option<&str>,
) -> Option<UnsavedAction> {
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("unsaved dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, trigger, changes, game, category));
    if response.should_close() {
        return Some(UnsavedAction::Cancel);
    }
    response.inner
}

fn contents(
    ui: &mut Ui,
    trigger: UnsavedTrigger,
    changes: usize,
    game: Option<&str>,
    category: Option<&str>,
) -> Option<UnsavedAction> {
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
    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(message(trigger, changes, game, category))
                .font(theme::body(14.0))
                .line_height(Some(21.0))
                .color(theme::TEXT),
        )
        .wrap(),
    );
    ui.add_space(20.0);
    // Cancel, Discard and Save, aligned right: added from the right.
    let size = vec2(WIDTH, BUTTONS_HEIGHT);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), |ui| {
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

/// The text naming how many settings of `game`, in `category`, changed, and
/// that `trigger` discards them. Without a game, it names neither.
fn message(
    trigger: UnsavedTrigger,
    changes: usize,
    game: Option<&str>,
    category: Option<&str>,
) -> String {
    let settings = match changes {
        1 => "1 setting".to_owned(),
        n => format!("{n} settings"),
    };
    let game = match (game, category) {
        (Some(game), Some(category)) => format!(" for {game} ({category})"),
        (Some(game), None) => format!(" for {game}"),
        (None, _) => String::new(),
    };
    let action = action(trigger);
    format!("You changed {settings}{game}. {action} without saving discards them.")
}

/// What `trigger` does, as the message says it.
fn action(trigger: UnsavedTrigger) -> &'static str {
    match trigger {
        UnsavedTrigger::Reload => "Reloading the auto splitter",
        UnsavedTrigger::Open => "Opening another auto splitter",
        UnsavedTrigger::ChangeGame => "Changing the game",
        UnsavedTrigger::ChangeCategory => "Changing the category",
        UnsavedTrigger::Close => "Closing the app",
    }
}

/// **Save and reload** for Reload, **Save** otherwise.
fn save_label(trigger: UnsavedTrigger) -> &'static str {
    match trigger {
        UnsavedTrigger::Reload => "Save and reload",
        UnsavedTrigger::Open
        | UnsavedTrigger::ChangeGame
        | UnsavedTrigger::ChangeCategory
        | UnsavedTrigger::Close => "Save",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_how_many_settings_of_the_game_changed() {
        assert_eq!(
            message(UnsavedTrigger::Reload, 1, Some("GTA"), None),
            "You changed 1 setting for GTA. Reloading the auto splitter without saving \
             discards them."
        );
        assert_eq!(
            message(UnsavedTrigger::Reload, 3, Some("GTA"), None),
            "You changed 3 settings for GTA. Reloading the auto splitter without saving \
             discards them."
        );
    }

    #[test]
    fn names_no_game_without_one() {
        assert_eq!(
            message(UnsavedTrigger::Close, 2, None, None),
            "You changed 2 settings. Closing the app without saving discards them."
        );
    }

    #[test]
    fn says_what_each_trigger_discards_them_by() {
        assert_eq!(
            message(UnsavedTrigger::Reload, 2, Some("GTA"), None),
            "You changed 2 settings for GTA. Reloading the auto splitter without saving \
             discards them."
        );
        assert_eq!(
            message(UnsavedTrigger::Open, 2, Some("GTA"), None),
            "You changed 2 settings for GTA. Opening another auto splitter without saving \
             discards them."
        );
        assert_eq!(
            message(UnsavedTrigger::ChangeGame, 2, Some("GTA"), None),
            "You changed 2 settings for GTA. Changing the game without saving discards them."
        );
        assert_eq!(
            message(UnsavedTrigger::Close, 2, Some("GTA"), None),
            "You changed 2 settings for GTA. Closing the app without saving discards them."
        );
    }

    #[test]
    fn names_the_category_after_the_game() {
        assert_eq!(
            message(
                UnsavedTrigger::ChangeCategory,
                3,
                Some("GTA San Andreas"),
                Some("Any%")
            ),
            "You changed 3 settings for GTA San Andreas (Any%). Changing the category without \
             saving discards them."
        );
    }

    #[test]
    fn changing_the_category_saves_with_save() {
        assert_eq!(save_label(UnsavedTrigger::ChangeCategory), "Save");
    }

    #[test]
    fn save_says_it_reloads_only_for_reload() {
        assert_eq!(save_label(UnsavedTrigger::Reload), "Save and reload");
        assert_eq!(save_label(UnsavedTrigger::Open), "Save");
        assert_eq!(save_label(UnsavedTrigger::ChangeGame), "Save");
        assert_eq!(save_label(UnsavedTrigger::Close), "Save");
    }
}
