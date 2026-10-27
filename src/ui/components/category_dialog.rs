//! Category dialog: picks which of the game's categories' settings to use,
//! and adds, renames and deletes categories (spec §6.8, §7.4).

#![expect(dead_code, reason = "the app opens this dialog in the next task")]

use eframe::egui::{
    self, Align, Frame, Id, Key, Label, Layout, Margin, Modal, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, TextEdit, Ui, UiBuilder, vec2,
};

use super::{
    BACKDROP,
    button::{Button, ButtonSize},
};
use crate::{config::Game, ui::theme};

/// The dialog's width inside its padding: the dialog is 480 px wide.
const WIDTH: f32 = 432.0;
/// The height of a row in the list, and of a name field.
const ROW_HEIGHT: f32 = 36.0;
/// The list scrolls beyond this height.
const LIST_HEIGHT: f32 = 240.0;

/// What was done in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryDialogAction {
    /// Use the category with this key.
    Use(String),
    /// Add a category with this name.
    Add(String),
    Rename {
        key: String,
        name: String,
    },
    /// Delete the category with this key.
    Delete(String),
    Cancel,
}

/// What the dialog is doing besides showing the list.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    List,
    /// A row is a name field. `focus` until the field has been focused.
    Renaming {
        key: String,
        name: String,
        focus: bool,
    },
    /// The new category's name field is shown.
    Adding {
        name: String,
        focus: bool,
    },
    /// Asking whether to delete the selected category.
    ConfirmDelete,
    /// "Name this category": asking which category the settings from before
    /// categories, in the category `key`, are for. `focus` until the field
    /// has been focused.
    Naming {
        key: String,
        name: String,
        focus: bool,
    },
}

/// The Category dialog's state.
#[derive(Debug, Clone)]
pub struct CategoryDialog {
    /// The game, as last read: its name, its categories and the one in use.
    game: Game,
    /// The key of the selected category.
    selected: String,
    mode: Mode,
}

impl CategoryDialog {
    /// The dialog for `game`, with the category in use selected.
    pub fn new(game: &Game) -> Self {
        Self {
            game: game.clone(),
            selected: game.current.clone(),
            mode: Mode::List,
        }
    }

    /// Shows `game` as it is now, after a category was added, renamed or
    /// deleted, and goes back to the list. Selects `select`, or keeps the
    /// selection if it still exists, or else the category in use.
    pub fn refresh(&mut self, game: &Game, select: Option<&str>) {
        let selected = select.unwrap_or(&self.selected).to_owned();
        self.game = game.clone();
        self.selected = if self.exists(&selected) {
            selected
        } else {
            game.current.clone()
        };
        self.mode = Mode::List;
    }

    /// "Name this category" for `game`'s category `key`, which holds the
    /// settings of a file from before categories (spec §6.8, §7.4).
    pub fn naming(game: &Game, key: &str) -> Self {
        Self {
            game: game.clone(),
            selected: key.to_owned(),
            mode: Mode::Naming {
                key: key.to_owned(),
                name: String::new(),
                focus: true,
            },
        }
    }

    /// Whether the dialog is asking for the name of settings from before
    /// categories.
    pub fn is_naming(&self) -> bool {
        matches!(self.mode, Mode::Naming { .. })
    }

    /// Leaves renaming, adding or confirming a delete. Naming has no list
    /// to go back to, so it stays.
    pub fn back_to_list(&mut self) {
        if !self.is_naming() {
            self.mode = Mode::List;
        }
    }

    fn exists(&self, key: &str) -> bool {
        self.game
            .categories
            .iter()
            .any(|category| category.key == key)
    }

    fn name_of<'a>(&'a self, key: &'a str) -> &'a str {
        self.game
            .categories
            .iter()
            .find(|category| category.key == key)
            .map_or(key, |category| category.name.as_str())
    }

    /// Whether `name` can be given to a category: not blank, and no other
    /// category than `except` has it.
    fn name_ok(&self, name: &str, except: Option<&str>) -> bool {
        !name.trim().is_empty() && !self.game.name_taken(name, except)
    }

    /// The category in use and the last one can't be deleted.
    fn can_delete(&self) -> bool {
        self.selected != self.game.current && self.game.categories.len() > 1
    }

    fn use_action(&self) -> CategoryDialogAction {
        CategoryDialogAction::Use(self.selected.clone())
    }

    fn start_rename(&mut self, key: &str) {
        self.selected = key.to_owned();
        self.mode = Mode::Renaming {
            key: key.to_owned(),
            name: self.name_of(key).to_owned(),
            focus: true,
        };
    }

    fn start_adding(&mut self) {
        self.mode = Mode::Adding {
            name: String::new(),
            focus: true,
        };
    }

    /// Escape, or a click outside: leaves an edit if there is one, and
    /// returns whether the dialog should close. Naming closes: it is Not now.
    fn escape(&mut self) -> bool {
        if self.mode == Mode::List || self.is_naming() {
            return true;
        }
        self.mode = Mode::List;
        false
    }
}

/// Shows the Category dialog over the window. Escape and clicking outside it
/// leave a rename, a new name or a delete question first, then cancel it.
pub fn category_dialog(
    ctx: &egui::Context,
    dialog: &mut CategoryDialog,
) -> Option<CategoryDialogAction> {
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("category dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, dialog));
    if response.should_close() {
        return dialog.escape().then_some(CategoryDialogAction::Cancel);
    }
    response.inner
}

fn contents(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    if dialog.is_naming() {
        return naming(ui, dialog);
    }
    ui.label(
        RichText::new("CATEGORY")
            .font(theme::display(16.0))
            .color(theme::TEXT),
    );
    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new(&dialog.game.name)
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
        )
        .truncate(),
    );

    ui.add_space(16.0);
    let mut action = list(ui, dialog);

    ui.add_space(12.0);
    action = action.or_else(|| adding(ui, dialog));
    if dialog.mode == Mode::ConfirmDelete {
        ui.add_space(12.0);
        action = action.or_else(|| confirm_delete(ui, dialog));
    }

    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(
                "Each category keeps its own settings. The category in use can't be deleted.",
            )
            .font(theme::body(12.0))
            .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );

    ui.add_space(20.0);
    let buttons = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let delete = ui.add(
                Button::secondary("Delete")
                    .enabled(dialog.can_delete() && dialog.mode == Mode::List),
            );
            if delete.clicked() {
                dialog.mode = Mode::ConfirmDelete;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let use_it = ui.add(Button::primary("Use this category"));
                let cancel = ui.add(Button::secondary("Cancel"));
                if use_it.clicked() {
                    Some(dialog.use_action())
                } else if cancel.clicked() {
                    Some(CategoryDialogAction::Cancel)
                } else {
                    None
                }
            })
            .inner
        })
        .inner;
    action.or(buttons)
}

/// The categories, each with Rename, or a name field for the one being
/// renamed.
fn list(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let mut action = None;
    let mut select = None;
    let mut rename = None;
    ScrollArea::vertical()
        .max_height(LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let categories = dialog.game.categories.clone();
            for category in &categories {
                if let Mode::Renaming { key, name, focus } = &mut dialog.mode
                    && *key == category.key
                {
                    let ok = !name.trim().is_empty() && !dialog.game.name_taken(name, Some(key));
                    if let Some(done) = rename_row(ui, name, focus, ok) {
                        action = done.then(|| CategoryDialogAction::Rename {
                            key: key.clone(),
                            name: name.trim().to_owned(),
                        });
                    }
                    continue;
                }
                let in_use = category.key == dialog.game.current;
                let selected = category.key == dialog.selected;
                match row(ui, &category.name, in_use, selected) {
                    Some(RowAction::Select) => select = Some(category.key.clone()),
                    Some(RowAction::Rename) => rename = Some(category.key.clone()),
                    None => {}
                }
            }
        });
    if let Some(key) = select {
        dialog.selected = key;
    }
    if let Some(key) = rename {
        dialog.start_rename(&key);
    }
    action
}

enum RowAction {
    Select,
    Rename,
}

/// A category in the list: its name, IN USE for the category in use, and
/// Rename. The selected row is outlined.
fn row(ui: &mut Ui, name: &str, in_use: bool, selected: bool) -> Option<RowAction> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if selected || response.hovered() {
        let stroke = if selected {
            Stroke::new(1.0, theme::ACCENT)
        } else {
            Stroke::NONE
        };
        ui.painter().rect(
            rect,
            theme::RADIUS,
            theme::RAISED,
            stroke,
            StrokeKind::Inside,
        );
    }
    let inner = rect.shrink2(vec2(12.0, 0.0));
    let rename = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let rename = ui.add(Button::secondary("Rename").size(ButtonSize::Xs));
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add(
                        Label::new(
                            RichText::new(name)
                                .font(theme::body(14.0))
                                .color(theme::TEXT),
                        )
                        .selectable(false)
                        .truncate(),
                    );
                    if in_use {
                        ui.add(
                            Label::new(
                                RichText::new("IN USE")
                                    .font(theme::mono(11.0))
                                    .color(theme::TEXT_SECONDARY),
                            )
                            .selectable(false),
                        );
                    }
                });
                rename.clicked()
            },
        )
        .inner;
    if rename {
        Some(RowAction::Rename)
    } else if response.clicked() {
        Some(RowAction::Select)
    } else {
        None
    }
}

/// The row of the category being renamed: its name in a field, with Done.
/// Returns `Some(true)` when the name is kept, by Done or Enter, while it is
/// `ok`.
fn rename_row(ui: &mut Ui, name: &mut String, focus: &mut bool, ok: bool) -> Option<bool> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let field = name_field(ui, name, focus, WIDTH - 72.0, "Category name");
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let done = ui.add(Button::secondary("Done").size(ButtonSize::Sm).enabled(ok));
        ((done.clicked() || entered) && ok).then_some(true)
    })
    .inner
}

/// The New category button, or the new category's name field with Add and a note
/// saying what it starts with.
fn adding(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let in_use = dialog.name_of(&dialog.game.current).to_owned();
    let ok = match &dialog.mode {
        Mode::Adding { name, .. } => dialog.name_ok(name, None),
        _ => false,
    };
    let Mode::Adding { name, focus } = &mut dialog.mode else {
        let new = ui.add(
            Button::secondary("+ New category")
                .size(ButtonSize::Sm)
                .enabled(dialog.mode == Mode::List),
        );
        if new.clicked() {
            dialog.start_adding();
        }
        return None;
    };
    let action = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let field = name_field(ui, name, focus, WIDTH - 64.0, "Category name");
            let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
            let add = ui.add(Button::secondary("Add").enabled(ok));
            ((add.clicked() || entered) && ok)
                .then(|| CategoryDialogAction::Add(name.trim().to_owned()))
        })
        .inner;
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new(format!(
                "Starts as a copy of the settings in use now ({in_use})."
            ))
            .font(theme::body(12.0))
            .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    action
}

/// "Name this category": the game, the question, a name field and Not now
/// and Save (spec §6.8).
fn naming(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let ok = match &dialog.mode {
        Mode::Naming { key, name, .. } => dialog.name_ok(name, Some(key)),
        _ => false,
    };
    let game = dialog.game.name.clone();
    let Mode::Naming { key, name, focus } = &mut dialog.mode else {
        return None;
    };
    ui.label(
        RichText::new("NAME THIS CATEGORY")
            .font(theme::display(16.0))
            .color(theme::TEXT),
    );
    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new(game)
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
        )
        .truncate(),
    );
    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(
                "These settings were saved before the bridge kept settings per category. \
                 Which category are they set up for?",
            )
            .font(theme::body(14.0))
            .color(theme::TEXT),
        )
        .wrap(),
    );
    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new("Category")
                .font(theme::body_semibold(13.0))
                .color(theme::TEXT),
        )
        .selectable(false),
    );
    ui.add_space(6.0);
    let field = name_field(ui, name, focus, WIDTH, "Any%");
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new("You can add more categories later. Not now keeps the name \"Default\".")
                .font(theme::body(12.0))
                .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    ui.add_space(20.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let save = ui.add(Button::primary("Save").enabled(ok));
        let not_now = ui.add(Button::secondary("Not now"));
        if (save.clicked() || entered) && ok {
            Some(CategoryDialogAction::Rename {
                key: key.clone(),
                name: name.trim().to_owned(),
            })
        } else if not_now.clicked() {
            Some(CategoryDialogAction::Cancel)
        } else {
            None
        }
    })
    .inner
}

/// A one-line name field `width` wide showing `hint` while empty, focused
/// the first time it shows.
fn name_field(
    ui: &mut Ui,
    name: &mut String,
    focus: &mut bool,
    width: f32,
    hint: &str,
) -> egui::Response {
    let field = ui.add(
        TextEdit::singleline(name)
            .font(theme::body(14.0))
            .hint_text(hint)
            .desired_width(width)
            .min_size(vec2(0.0, 32.0))
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(10, 0)),
    );
    if *focus {
        field.request_focus();
        *focus = false;
    }
    field
}

/// "Delete <category> and its settings?", with Keep and Delete.
fn confirm_delete(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let name = dialog.name_of(&dialog.selected).to_owned();
    let frame = Frame::NONE
        .fill(theme::WINDOW)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(12));
    frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                Label::new(
                    RichText::new(format!(
                        "Delete {name} and its settings? This can't be undone."
                    ))
                    .font(theme::body(13.0))
                    .color(theme::TEXT),
                )
                .wrap(),
            );
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let delete = ui.add(Button::primary("Delete").size(ButtonSize::Sm));
                let keep = ui.add(Button::secondary("Keep").size(ButtonSize::Sm));
                if keep.clicked() {
                    dialog.back_to_list();
                }
                delete
                    .clicked()
                    .then(|| CategoryDialogAction::Delete(dialog.selected.clone()))
            })
            .inner
        })
        .inner
}

#[cfg(test)]
mod tests {
    use toml::Table;

    use super::*;
    use crate::config::RunCategory;

    fn game() -> Game {
        let category = |key: &str, name: &str| RunCategory {
            key: key.to_owned(),
            name: name.to_owned(),
            settings: Table::new(),
        };
        Game {
            slug: "gta".to_owned(),
            name: "GTA".to_owned(),
            categories: vec![category("100", "100%"), category("any", "Any%")],
            current: "any".to_owned(),
            unnamed: None,
        }
    }

    #[test]
    fn naming_asks_for_a_new_name_and_escape_is_not_now() {
        let mut dialog = CategoryDialog::naming(&game(), "any");
        assert!(dialog.is_naming());
        assert_eq!(
            dialog.mode,
            Mode::Naming {
                key: "any".to_owned(),
                name: String::new(),
                focus: true
            }
        );
        // The category being named may keep its own name.
        assert!(dialog.name_ok("ANY%", Some("any")));
        assert!(!dialog.name_ok("100%", Some("any")));
        dialog.back_to_list();
        assert!(dialog.is_naming());
        assert!(dialog.escape());
    }

    #[test]
    fn opens_on_the_category_in_use() {
        let dialog = CategoryDialog::new(&game());
        assert_eq!(dialog.selected, "any");
        assert!(!dialog.can_delete());
        assert_eq!(
            dialog.use_action(),
            CategoryDialogAction::Use("any".to_owned())
        );
    }

    #[test]
    fn only_another_category_than_the_one_in_use_can_be_deleted() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.selected = "100".to_owned();
        assert!(dialog.can_delete());
        let mut one = game();
        one.categories.retain(|category| category.key == "any");
        let dialog = CategoryDialog::new(&one);
        assert!(!dialog.can_delete());
    }

    #[test]
    fn a_new_name_must_be_new_and_not_blank() {
        let dialog = CategoryDialog::new(&game());
        assert!(dialog.name_ok("Low%", None));
        assert!(!dialog.name_ok(" any% ", None));
        assert!(!dialog.name_ok("  ", None));
        assert!(dialog.name_ok("ANY%", Some("any")));
    }

    #[test]
    fn renaming_starts_from_the_name() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_rename("100");
        assert_eq!(
            dialog.mode,
            Mode::Renaming {
                key: "100".to_owned(),
                name: "100%".to_owned(),
                focus: true
            }
        );
    }

    #[test]
    fn escape_leaves_an_edit_before_it_closes_the_dialog() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_adding();
        assert!(!dialog.escape());
        assert_eq!(dialog.mode, Mode::List);
        assert!(dialog.escape());
    }

    #[test]
    fn refreshing_selects_the_category_asked_for_or_keeps_one_that_exists() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_adding();
        let mut more = game();
        more.categories.push(RunCategory {
            key: "low".to_owned(),
            name: "Low%".to_owned(),
            settings: Table::new(),
        });
        dialog.refresh(&more, Some("low"));
        assert_eq!(dialog.selected, "low");
        assert_eq!(dialog.mode, Mode::List);
        dialog.refresh(&game(), None);
        // "low" is gone: back to the category in use.
        assert_eq!(dialog.selected, "any");
    }
}
