//! Category dialog: picks which of the game's categories' settings to use,
//! and adds, renames and deletes categories (spec §6.8, §7.4). For a linked
//! game, its name fields also offer the game's categories from speedrun.com
//! and link a category to one (spec §7.5.1).

use std::fmt;

use eframe::egui::{
    self, Align, Frame, Id, Key, Label, Layout, Margin, Modal, Modifiers, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, Ui, UiBuilder, vec2,
};

use super::{
    BACKDROP,
    button::{Button, ButtonSize},
    category_suggest::{Clicked, Row, Rows, Shown, Suggest, offered, under_field},
    game_dialog::POLL_EVERY,
};
use crate::{
    config::Game,
    speedrun::{CategoryLink, CategorySource, Variant, clear_variant},
    ui::theme,
};

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
    /// Add a category with this name, linked to this speedrun.com variant.
    Add {
        name: String,
        link: Option<CategoryLink>,
    },
    /// Rename the category with this key, and link it to this speedrun.com
    /// variant or unlink it.
    Rename {
        key: String,
        name: String,
        link: Option<CategoryLink>,
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
pub struct CategoryDialog {
    /// The game, as last read: its name, its categories and the one in use.
    game: Game,
    /// The key of the selected category.
    selected: String,
    mode: Mode,
    /// Fetches the game's variants from speedrun.com.
    source: Box<dyn CategorySource>,
    /// The speedrun.com game whose variants were asked for, when the game is
    /// linked. Answers for any other game are ignored.
    asked: Option<String>,
    /// Whether the answer hasn't come back yet.
    awaiting: bool,
    /// The game's variants, once they arrive, kept while the dialog is open.
    variants: Vec<Variant>,
    /// The link and the list of the open name field.
    suggest: Suggest,
    /// The error of the request, until it is taken.
    failure: Option<String>,
    /// Whether the error was taken, so it is reported only once.
    failure_taken: bool,
}

impl fmt::Debug for CategoryDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CategoryDialog")
            .field("game", &self.game.slug)
            .field("selected", &self.selected)
            .field("mode", &self.mode)
            .field("asked", &self.asked)
            .field("variants", &self.variants.len())
            .finish_non_exhaustive()
    }
}

impl CategoryDialog {
    /// The dialog for `game`, with the category in use selected. `source` is
    /// asked for the game's variants when the game is linked.
    pub fn new(game: &Game, source: Box<dyn CategorySource>) -> Self {
        let mut source = source;
        let asked = game.speedrun_id.clone();
        if let Some(id) = &asked {
            source.request(id);
        }
        Self {
            game: game.clone(),
            selected: game.current.clone(),
            mode: Mode::List,
            source,
            awaiting: asked.is_some(),
            asked,
            variants: Vec::new(),
            suggest: Suggest::default(),
            failure: None,
            failure_taken: false,
        }
    }

    /// Shows `game` as it is now, after a category was added, renamed or
    /// deleted, and goes back to the list. Selects `select`, or keeps the
    /// selection if it still exists, or else the category in use. The
    /// variants are kept.
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
    /// settings of a file from before categories (spec §6.8, §7.4). `source`
    /// is asked for the game's variants when the game is linked.
    pub fn naming(game: &Game, key: &str, source: Box<dyn CategorySource>) -> Self {
        let mut dialog = Self::new(game, source);
        dialog.selected = key.to_owned();
        dialog.mode = Mode::Naming {
            key: key.to_owned(),
            name: String::new(),
            focus: true,
        };
        dialog.open_field(None, false);
        dialog
    }

    /// The error of the request for the variants, once per dialog.
    #[cfg_attr(not(test), allow(dead_code))] // Logged by the app, which comes later.
    pub fn take_failure(&mut self) -> Option<String> {
        let failure = self.failure.take();
        self.failure_taken |= failure.is_some();
        failure
    }

    /// Whether the game is linked, so its name fields offer its variants.
    fn linked(&self) -> bool {
        self.asked.is_some()
    }

    /// Runs every frame: takes in the answer for the game's variants.
    fn tick(&mut self) {
        while let Some((id, answer)) = self.source.poll() {
            if self.asked.as_deref() != Some(id.as_str()) {
                continue;
            }
            self.awaiting = false;
            match answer {
                Ok(variants) => {
                    self.variants = variants;
                    self.link_on_own();
                }
                Err(error) => {
                    if self.failure.is_none() && !self.failure_taken {
                        self.failure = Some(error);
                    }
                }
            }
        }
    }

    /// Whether any variant is a level's, so the level box shows.
    fn has_levels(&self) -> bool {
        self.variants.iter().any(Variant::is_level)
    }

    /// The name in the open name field.
    fn name(&self) -> Option<&str> {
        match &self.mode {
            Mode::Renaming { name, .. } | Mode::Adding { name, .. } | Mode::Naming { name, .. } => {
                Some(name)
            }
            Mode::List | Mode::ConfirmDelete => None,
        }
    }

    fn name_mut(&mut self) -> Option<&mut String> {
        match &mut self.mode {
            Mode::Renaming { name, .. } | Mode::Adding { name, .. } | Mode::Naming { name, .. } => {
                Some(name)
            }
            Mode::List | Mode::ConfirmDelete => None,
        }
    }

    /// The category being renamed or named.
    fn editing_key(&self) -> Option<&str> {
        match &self.mode {
            Mode::Renaming { key, .. } | Mode::Naming { key, .. } => Some(key),
            _ => None,
        }
    }

    /// Gives the open name field its focus back.
    fn refocus(&mut self) {
        if let Mode::Renaming { focus, .. }
        | Mode::Adding { focus, .. }
        | Mode::Naming { focus, .. } = &mut self.mode
        {
            *focus = true;
        }
    }

    /// A name field opens, linked to `link`. Names declined earlier in the
    /// dialog stay declined.
    fn open_field(&mut self, link: Option<CategoryLink>, may_link_on_own: bool) {
        let declined = std::mem::take(&mut self.suggest.declined);
        self.suggest = Suggest::open(link, may_link_on_own, declined);
        self.link_on_own();
    }

    /// What the list shows for the name in the field.
    fn rows(&self) -> Rows {
        Suggest::rows(
            &self.variants,
            self.suggest.levels,
            self.name().unwrap_or_default(),
        )
    }

    /// The number of rows of the list, counting the last one, which shows once
    /// something is typed.
    fn row_count(&self) -> usize {
        match self.rows() {
            Rows::TypeToSearch(_) => 0,
            Rows::Variants(rows) => {
                rows.len() + usize::from(!self.name().unwrap_or_default().trim().is_empty())
            }
        }
    }

    /// The key of another category than `except` that is linked to
    /// `variant`, or else that has its name, ignoring case and surrounding
    /// spaces.
    fn taken_by(&self, variant: &Variant, except: Option<&str>) -> Option<String> {
        let others = || {
            self.game
                .categories
                .iter()
                .filter(move |category| Some(category.key.as_str()) != except)
        };
        let name = variant.name.trim().to_lowercase();
        others()
            .find(|category| category.speedrun.as_ref() == Some(&variant.link))
            .or_else(|| others().find(|category| category.name.trim().to_lowercase() == name))
            .map(|category| category.key.clone())
    }

    /// Whether `variant` can't be picked: while renaming or naming, another
    /// category has it.
    fn dimmed(&self, variant: &Variant) -> bool {
        self.editing_key()
            .is_some_and(|key| self.taken_by(variant, Some(key)).is_some())
    }

    /// Picks the list's variant row `row`. It fills the field with the
    /// variant's name and links it; while adding, a variant another category
    /// has renames and links that category instead, and selects it. A dimmed
    /// row does nothing.
    fn pick(&mut self, row: usize) -> Option<CategoryDialogAction> {
        let Rows::Variants(rows) = self.rows() else {
            return None;
        };
        let variant = self.variants.get(*rows.get(row)?)?.clone();
        if self.dimmed(&variant) {
            return None;
        }
        if matches!(self.mode, Mode::Adding { .. })
            && let Some(key) = self.taken_by(&variant, None)
        {
            self.selected.clone_from(&key);
            return Some(CategoryDialogAction::Rename {
                key,
                name: variant.name,
                link: Some(variant.link),
            });
        }
        self.fill(variant);
        None
    }

    /// Fills the field with `variant`'s name and links it.
    fn fill(&mut self, variant: Variant) {
        if let Some(name) = self.name_mut() {
            *name = variant.name;
        }
        self.suggest.pick(variant.link);
    }

    /// The list's last row: keeps the name typed, unlinked.
    fn pick_typed(&mut self) {
        let name = self.name().unwrap_or_default().to_owned();
        self.suggest.pick_typed(&name);
    }

    /// Enter on the highlighted row `row`: the last row, or a variant.
    fn pick_highlighted(&mut self, row: usize) -> Option<CategoryDialogAction> {
        if row + 1 == self.row_count() && !self.name().unwrap_or_default().trim().is_empty() {
            self.pick_typed();
            None
        } else {
            self.pick(row)
        }
    }

    /// Links the name in the field to the variant that clearly matches it,
    /// once the variants are in, if it may: only a category's own name, when
    /// it is renamed unlinked and the name wasn't declined (spec §7.5.1).
    fn link_on_own(&mut self) {
        if !self.suggest.may_link_on_own || self.variants.is_empty() {
            return;
        }
        self.suggest.may_link_on_own = false;
        let Some(name) = self.name().map(str::to_owned) else {
            return;
        };
        if self.suggest.link.is_some() || self.suggest.declined(&name) {
            return;
        }
        let offered: Vec<&Variant> = offered(&self.variants, self.suggest.levels)
            .into_iter()
            .map(|index| &self.variants[index])
            .collect();
        let Some(found) = clear_variant(&name, &offered) else {
            return;
        };
        let variant = offered[found].clone();
        if !self.dimmed(&variant) {
            self.fill(variant);
        }
    }

    /// The Add the new name field would give, or `None` while its name can't
    /// be used.
    fn add_action(&self) -> Option<CategoryDialogAction> {
        let Mode::Adding { name, .. } = &self.mode else {
            return None;
        };
        self.name_ok(name, None).then(|| CategoryDialogAction::Add {
            name: name.trim().to_owned(),
            link: self.suggest.link.clone(),
        })
    }

    /// The Rename that Done or Save would give, or `None` while the name
    /// can't be used.
    fn rename_action(&self) -> Option<CategoryDialogAction> {
        let (Mode::Renaming { key, name, .. } | Mode::Naming { key, name, .. }) = &self.mode else {
            return None;
        };
        self.name_ok(name, Some(key))
            .then(|| CategoryDialogAction::Rename {
                key: key.clone(),
                name: name.trim().to_owned(),
                link: self.suggest.link.clone(),
            })
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

    /// Whether the rows can be selected and renamed: not while a rename, a
    /// new name or a delete question is open, so the selection can't change
    /// under them.
    fn rows_active(&self) -> bool {
        self.mode == Mode::List
    }

    /// The Delete the confirm question returns, or `None` for a category
    /// that can't be deleted.
    fn confirmed_delete(&self) -> Option<CategoryDialogAction> {
        self.can_delete()
            .then(|| CategoryDialogAction::Delete(self.selected.clone()))
    }

    fn use_action(&self) -> CategoryDialogAction {
        CategoryDialogAction::Use(self.selected.clone())
    }

    /// Renames `key`, starting from its name and its link. An unlinked
    /// category's name wasn't typed, so a clear match links it on its own.
    fn start_rename(&mut self, key: &str) {
        self.selected = key.to_owned();
        self.mode = Mode::Renaming {
            key: key.to_owned(),
            name: self.name_of(key).to_owned(),
            focus: true,
        };
        let link = self
            .game
            .categories
            .iter()
            .find(|category| category.key == key)
            .and_then(|category| category.speedrun.clone());
        let may_link_on_own = link.is_none();
        self.open_field(link, may_link_on_own);
    }

    fn start_adding(&mut self) {
        self.mode = Mode::Adding {
            name: String::new(),
            focus: true,
        };
        self.open_field(None, false);
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
/// leave a rename, a new name or a delete question first, then cancel it;
/// with the list of speedrun.com categories open, Escape closes the list.
pub fn category_dialog(
    ctx: &egui::Context,
    dialog: &mut CategoryDialog,
) -> Option<CategoryDialogAction> {
    dialog.tick();
    if dialog.awaiting {
        ctx.request_repaint_after(POLL_EVERY);
    }
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
                let use_it =
                    ui.add(Button::primary("Use this category").enabled(dialog.mode == Mode::List));
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
    let active = dialog.rows_active();
    let mut select = None;
    let mut rename = None;
    ScrollArea::vertical()
        .max_height(LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let categories = dialog.game.categories.clone();
            for category in &categories {
                if matches!(&dialog.mode, Mode::Renaming { key, .. } if *key == category.key) {
                    action = rename_row(ui, dialog);
                    continue;
                }
                let in_use = category.key == dialog.game.current;
                let selected = category.key == dialog.selected;
                match row(ui, &category.name, in_use, selected, active) {
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
fn row(ui: &mut Ui, name: &str, in_use: bool, selected: bool, active: bool) -> Option<RowAction> {
    let sense = if active {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), sense);
    if selected || (active && response.hovered()) {
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
                let rename = ui.add(
                    Button::secondary("Rename")
                        .size(ButtonSize::Xs)
                        .enabled(active),
                );
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

/// The row of the category being renamed: its name in a field, with Done,
/// and under it the speedrun.com suggestions. Done or Enter keeps the name
/// while it can be used.
fn rename_row(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let picked = list_keys(ui, dialog);
    let ok = dialog.rename_action().is_some();
    let (field, done) = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let Mode::Renaming { name, focus, .. } = &mut dialog.mode else {
                unreachable!("the row being renamed");
            };
            let field = name_field(ui, name, focus, WIDTH - 72.0, "Category name");
            let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
            let done = ui.add(Button::secondary("Done").size(ButtonSize::Sm).enabled(ok));
            let done = done.clicked() || entered;
            (field, done)
        })
        .inner;
    if field.changed() {
        dialog.suggest.typed();
    }
    let done = done.then(|| dialog.rename_action()).flatten();
    let under = suggestions(ui, dialog, field.rect, field.rect.width());
    picked.or(done).or(under)
}

/// Feeds the keys pressed this frame to the open list of speedrun.com
/// categories, taking the ones it uses, so Escape closes the list rather
/// than leaving the field, and an Enter that picks a row isn't also Add,
/// Done or Save. Returns what picking the row gave.
fn list_keys(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    if !dialog.linked() {
        return None;
    }
    let mut action = None;
    for key in [Key::ArrowDown, Key::ArrowUp, Key::Enter, Key::Escape] {
        let rows = dialog.row_count();
        let highlight = dialog.suggest.highlight;
        let used = ui.input_mut(|input| {
            let used = input.modifiers.is_none()
                && input.key_pressed(key)
                && dialog.suggest.key(key, rows);
            if used {
                input.consume_key(Modifiers::NONE, key);
            }
            used
        });
        if !used {
            continue;
        }
        if key == Key::Enter
            && let Some(row) = highlight
        {
            action = dialog.pick_highlighted(row);
        }
        // Escape takes the field's focus; it gets it back.
        if matches!(key, Key::Enter | Key::Escape) {
            dialog.refocus();
        }
    }
    action
}

/// Under a name field `field` of a linked game: the linked line, the level
/// box and the list of speedrun.com categories, `width` wide. Returns what
/// picking a row gave.
fn suggestions(
    ui: &mut Ui,
    dialog: &mut CategoryDialog,
    field: Rect,
    width: f32,
) -> Option<CategoryDialogAction> {
    if !dialog.linked() {
        return None;
    }
    let rows = dialog.rows();
    let has_levels = dialog.has_levels();
    let typed = dialog.name().unwrap_or_default().to_owned();
    // Each listed variant's index, whether it is set up, and whether it is
    // dimmed, worked out before the rows borrow the variants' names.
    let flags: Vec<(usize, bool, bool)> = match &rows {
        Rows::TypeToSearch(_) => Vec::new(),
        Rows::Variants(rows) => rows
            .iter()
            .map(|&index| {
                let variant = &dialog.variants[index];
                let set_up = dialog.game.linked_to(&variant.link).is_some();
                (index, set_up, dialog.dimmed(variant))
            })
            .collect(),
    };
    let shown = match rows {
        Rows::TypeToSearch(count) => Shown::TypeToSearch(count),
        Rows::Variants(_) => Shown::Variants(
            flags
                .into_iter()
                .map(|(index, set_up, dimmed)| Row {
                    name: &dialog.variants[index].name,
                    set_up,
                    dimmed,
                })
                .collect(),
        ),
    };
    let under = under_field(
        ui,
        field,
        width,
        &mut dialog.suggest,
        has_levels,
        &shown,
        &typed,
    );
    if under.refocus {
        dialog.refocus();
    }
    match under.clicked? {
        Clicked::Row(row) => dialog.pick(row),
        Clicked::Typed => {
            dialog.pick_typed();
            None
        }
    }
}

/// The New category button, or the new category's name field with Add and a note
/// saying what it starts with.
fn adding(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let in_use = dialog.name_of(&dialog.game.current).to_owned();
    let ok = dialog.add_action().is_some();
    if !matches!(dialog.mode, Mode::Adding { .. }) {
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
    let picked = list_keys(ui, dialog);
    let (field, add) = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let Mode::Adding { name, focus } = &mut dialog.mode else {
                unreachable!("adding");
            };
            let field = name_field(ui, name, focus, WIDTH - 64.0, "Category name");
            let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
            let add = ui.add(Button::secondary("Add").enabled(ok));
            let add = add.clicked() || entered;
            (field, add)
        })
        .inner;
    if field.changed() {
        dialog.suggest.typed();
    }
    let add = add.then(|| dialog.add_action()).flatten();
    let under = suggestions(ui, dialog, field.rect, WIDTH);
    let action = picked.or(add).or(under);
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
    let ok = dialog.rename_action().is_some();
    let game = dialog.game.name.clone();
    let note = match &dialog.mode {
        Mode::Naming { key, .. } => format!(
            "You can add more categories later. Not now keeps the name \"{}\".",
            dialog.name_of(key)
        ),
        _ => String::new(),
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
    let picked = list_keys(ui, dialog);
    let Mode::Naming { name, focus, .. } = &mut dialog.mode else {
        return None;
    };
    let field = name_field(ui, name, focus, WIDTH, "Any%");
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    if field.changed() {
        dialog.suggest.typed();
    }
    let under = suggestions(ui, dialog, field.rect, WIDTH);
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new(note)
                .font(theme::body(12.0))
                .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    ui.add_space(20.0);
    let buttons = ui
        .with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let save = ui.add(Button::primary("Save").enabled(ok));
            let not_now = ui.add(Button::secondary("Not now"));
            if save.clicked() || entered {
                dialog.rename_action()
            } else if not_now.clicked() {
                Some(CategoryDialogAction::Cancel)
            } else {
                None
            }
        })
        .inner;
    picked.or(under).or(buttons)
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
                if delete.clicked() {
                    dialog.confirmed_delete()
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
    use toml::Table;

    use std::collections::BTreeMap;

    use eframe::egui::{Context, Event, Modifiers, Pos2, RawInput, Rect};

    use super::*;
    use crate::config::RunCategory;
    use crate::speedrun::{FakeCategories, OfflineCategories};

    fn game() -> Game {
        let category = |key: &str, name: &str| RunCategory {
            key: key.to_owned(),
            name: name.to_owned(),
            settings: Table::new(),
            speedrun: None,
        };
        Game {
            slug: "gta".to_owned(),
            name: "GTA".to_owned(),
            speedrun_id: None,
            categories: vec![category("100", "100%"), category("any", "Any%")],
            current: "any".to_owned(),
            unnamed: None,
        }
    }

    #[test]
    fn naming_asks_for_a_new_name_and_escape_is_not_now() {
        let mut dialog = CategoryDialog::naming(&game(), "any", Box::new(OfflineCategories));
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
        let dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        assert_eq!(dialog.selected, "any");
        assert!(!dialog.can_delete());
        assert_eq!(
            dialog.use_action(),
            CategoryDialogAction::Use("any".to_owned())
        );
    }

    #[test]
    fn only_another_category_than_the_one_in_use_can_be_deleted() {
        let mut dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        dialog.selected = "100".to_owned();
        assert!(dialog.can_delete());
        let mut one = game();
        one.categories.retain(|category| category.key == "any");
        let dialog = CategoryDialog::new(&one, Box::new(OfflineCategories));
        assert!(!dialog.can_delete());
    }

    #[test]
    fn rows_are_inactive_and_delete_is_refused_for_the_category_in_use() {
        let mut dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        assert!(dialog.rows_active());
        dialog.selected = "100".to_owned();
        dialog.mode = Mode::ConfirmDelete;
        assert!(!dialog.rows_active());
        assert_eq!(
            dialog.confirmed_delete(),
            Some(CategoryDialogAction::Delete("100".to_owned()))
        );
        // Even if the selection reached the category in use, nothing is deleted.
        dialog.selected = "any".to_owned();
        assert_eq!(dialog.confirmed_delete(), None);
        dialog.start_adding();
        assert!(!dialog.rows_active());
    }

    #[test]
    fn a_new_name_must_be_new_and_not_blank() {
        let dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        assert!(dialog.name_ok("Low%", None));
        assert!(!dialog.name_ok(" any% ", None));
        assert!(!dialog.name_ok("  ", None));
        assert!(dialog.name_ok("ANY%", Some("any")));
    }

    #[test]
    fn renaming_starts_from_the_name() {
        let mut dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
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
        let mut dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        dialog.start_adding();
        assert!(!dialog.escape());
        assert_eq!(dialog.mode, Mode::List);
        assert!(dialog.escape());
    }

    #[test]
    fn refreshing_selects_the_category_asked_for_or_keeps_one_that_exists() {
        let mut dialog = CategoryDialog::new(&game(), Box::new(OfflineCategories));
        dialog.start_adding();
        let mut more = game();
        more.categories.push(RunCategory {
            key: "low".to_owned(),
            name: "Low%".to_owned(),
            settings: Table::new(),
            speedrun: None,
        });
        dialog.refresh(&more, Some("low"));
        assert_eq!(dialog.selected, "low");
        assert_eq!(dialog.mode, Mode::List);
        dialog.refresh(&game(), None);
        // "low" is gone: back to the category in use.
        assert_eq!(dialog.selected, "any");
    }

    fn variant(name: &str, id: &str, value: &str, level: Option<&str>) -> Variant {
        Variant {
            name: name.to_owned(),
            link: CategoryLink {
                id: id.to_owned(),
                values: BTreeMap::from([("platform".to_owned(), value.to_owned())]),
                level: level.map(str::to_owned),
            },
        }
    }

    fn pc() -> Variant {
        variant("Any% (PC)", "any", "pc", None)
    }

    fn n64() -> Variant {
        variant("Any% (N64)", "any", "n64", None)
    }

    fn stage() -> Variant {
        variant(
            "Bob-omb Battlefield: Stage RTA (N64)",
            "stage",
            "n64",
            Some("bob"),
        )
    }

    /// The test game, linked to the speedrun.com game "g".
    fn linked_game() -> Game {
        Game {
            speedrun_id: Some("g".to_owned()),
            ..game()
        }
    }

    /// `game` with its category `key` named `name` and linked to `link`.
    fn with(mut game: Game, key: &str, name: &str, link: Option<CategoryLink>) -> Game {
        let category = game
            .categories
            .iter_mut()
            .find(|category| category.key == key)
            .unwrap();
        category.name = name.to_owned();
        category.speedrun = link;
        game
    }

    /// The Category dialog for `game`, asking a fake, and the fake to drive it.
    fn open(game: &Game) -> (CategoryDialog, FakeCategories) {
        let fake = FakeCategories::default();
        let dialog = CategoryDialog::new(game, Box::new(fake.clone()));
        (dialog, fake)
    }

    /// The variants of "g" arrive.
    fn arrive(dialog: &mut CategoryDialog, fake: &FakeCategories, variants: Vec<Variant>) {
        fake.answer("g", Ok(variants));
        dialog.tick();
    }

    /// Types `text` into the open name field.
    fn type_in(dialog: &mut CategoryDialog, text: &str) {
        *dialog.name_mut().unwrap() = text.to_owned();
        dialog.suggest.typed();
    }

    fn name(dialog: &CategoryDialog) -> &str {
        dialog.name().unwrap()
    }

    #[test]
    fn asks_for_the_variants_only_for_a_linked_game() {
        let (dialog, fake) = open(&linked_game());
        assert_eq!(fake.requests(), ["g"]);
        assert!(dialog.linked());
        let (mut dialog, fake) = open(&game());
        dialog.start_adding();
        dialog.tick();
        assert!(fake.requests().is_empty());
        assert!(!dialog.linked(), "a game that isn't linked has no list");
    }

    #[test]
    fn an_answer_for_another_game_is_ignored() {
        let (mut dialog, fake) = open(&linked_game());
        fake.answer("other", Ok(vec![pc()]));
        dialog.tick();
        assert!(dialog.variants.is_empty());
        assert!(dialog.awaiting);
        arrive(&mut dialog, &fake, vec![pc()]);
        assert_eq!(dialog.variants, [pc()]);
        assert!(!dialog.awaiting);
    }

    #[test]
    fn the_variants_are_kept_across_a_refresh() {
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc()]);
        dialog.refresh(&linked_game(), None);
        assert_eq!(dialog.variants, [pc()]);
        assert_eq!(fake.requests(), ["g"]);
    }

    #[test]
    fn picking_fills_the_name_and_links_it() {
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_adding();
        assert!(dialog.suggest.list_open);
        assert_eq!(dialog.rows(), Rows::Variants(vec![0, 1]));
        assert_eq!(dialog.pick(1), None);
        assert_eq!(name(&dialog), "Any% (N64)");
        assert_eq!(dialog.suggest.link, Some(n64().link));
        assert!(!dialog.suggest.list_open);
    }

    #[test]
    fn typing_removes_the_link() {
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_adding();
        dialog.pick(0);
        type_in(&mut dialog, "Any% (PC) glitchless");
        assert_eq!(dialog.suggest.link, None);
        assert!(dialog.suggest.list_open);
        assert_eq!(dialog.rows(), Rows::Variants(vec![]));
    }

    #[test]
    fn change_removes_the_link_and_opens_the_list() {
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_adding();
        dialog.pick(0);
        dialog.suggest.change_link();
        assert_eq!(dialog.suggest.link, None);
        assert!(dialog.suggest.list_open);
        assert_eq!(name(&dialog), "Any% (PC)");
        assert_eq!(dialog.rows(), Rows::Variants(vec![0]));
    }

    #[test]
    fn the_last_row_keeps_the_typed_name_unlinked_and_is_not_linked_again() {
        let game = with(linked_game(), "100", "any% pc", None);
        let (mut dialog, fake) = open(&game);
        dialog.start_rename("100");
        assert_eq!(dialog.row_count(), 1, "only the last row before the answer");
        dialog.pick_typed();
        assert!(!dialog.suggest.list_open);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        assert_eq!(dialog.suggest.link, None);
        assert_eq!(name(&dialog), "any% pc");
        // Renaming it again while the dialog is open doesn't link it either.
        dialog.refresh(&game, None);
        dialog.start_rename("100");
        assert_eq!(dialog.suggest.link, None);
        assert_eq!(name(&dialog), "any% pc");
        assert!(dialog.suggest.list_open);
    }

    #[test]
    fn add_returns_the_link() {
        let (ctx, mut dialog) = adding_on_screen();
        dialog.pick(0);
        assert_eq!(
            frame(&ctx, &mut dialog, press(Key::Enter)),
            Some(CategoryDialogAction::Add {
                name: "Any% (PC)".to_owned(),
                link: Some(pc().link)
            })
        );
    }

    #[test]
    fn a_typed_name_on_a_linked_game_adds_without_a_link() {
        let (ctx, mut dialog) = adding_on_screen();
        type_in(&mut dialog, "My Route%");
        let add = Some(CategoryDialogAction::Add {
            name: "My Route%".to_owned(),
            link: None,
        });
        assert_eq!(dialog.add_action(), add);
        dialog.pick_typed();
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Enter)), add);
    }

    #[test]
    fn picking_a_set_up_variant_while_adding_uses_that_category() {
        let game = with(linked_game(), "any", "Old", Some(pc().link));
        let game = with(game, "100", "ANY% (n64)", None);
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_adding();
        assert_eq!(
            dialog.pick(0),
            Some(CategoryDialogAction::Rename {
                key: "any".to_owned(),
                name: "Any% (PC)".to_owned(),
                link: Some(pc().link)
            })
        );
        assert_eq!(dialog.selected, "any");
        dialog.start_adding();
        assert_eq!(
            dialog.pick(1),
            Some(CategoryDialogAction::Rename {
                key: "100".to_owned(),
                name: "Any% (N64)".to_owned(),
                link: Some(n64().link)
            })
        );
        assert_eq!(dialog.selected, "100");
    }

    #[test]
    fn rows_another_category_has_are_dimmed_while_renaming() {
        let game = with(linked_game(), "any", "Old", Some(pc().link));
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("100");
        assert!(dialog.dimmed(&pc()));
        assert!(!dialog.dimmed(&n64()));
        assert_eq!(dialog.pick(0), None);
        assert_eq!(name(&dialog), "100%");
        assert_eq!(dialog.suggest.link, None);
        // A variant named like another category is dimmed too.
        let game = with(linked_game(), "any", "any% (n64)", None);
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("100");
        assert!(dialog.dimmed(&n64()));
        // The category being renamed may take its own variant.
        let game = with(linked_game(), "any", "Old", Some(pc().link));
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("any");
        assert!(!dialog.dimmed(&pc()));
    }

    #[test]
    fn rename_on_a_linked_category_starts_linked() {
        let game = with(linked_game(), "any", "Any% (PC)", Some(pc().link));
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("any");
        assert_eq!(dialog.suggest.link, Some(pc().link));
        assert!(!dialog.suggest.list_open);
        assert_eq!(
            dialog.rename_action(),
            Some(CategoryDialogAction::Rename {
                key: "any".to_owned(),
                name: "Any% (PC)".to_owned(),
                link: Some(pc().link)
            })
        );
    }

    #[test]
    fn rename_on_an_unlinked_category_links_a_clear_match_on_its_own() {
        let game = with(linked_game(), "100", "any% pc", None);
        // Variants arriving after the rename opened.
        let (mut dialog, fake) = open(&game);
        dialog.start_rename("100");
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        assert_eq!(name(&dialog), "Any% (PC)");
        assert_eq!(dialog.suggest.link, Some(pc().link));
        // Variants already in when the rename opens.
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("100");
        assert_eq!(dialog.suggest.link, Some(pc().link));
        // A name without a clear match opens the list.
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_rename("any");
        assert_eq!(dialog.suggest.link, None);
        assert!(dialog.suggest.list_open);
        // The empty fields never link on their own.
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc()]);
        dialog.start_adding();
        assert_eq!(dialog.suggest.link, None);
    }

    #[test]
    fn the_level_box_starts_ticked_for_a_level_linked_category() {
        let game = with(linked_game(), "any", "Stage", Some(stage().link));
        let (mut dialog, fake) = open(&game);
        arrive(&mut dialog, &fake, vec![pc(), stage()]);
        assert!(dialog.has_levels());
        dialog.start_rename("any");
        assert!(dialog.suggest.levels);
        dialog.start_rename("100");
        assert!(!dialog.suggest.levels);
        dialog.start_adding();
        assert!(!dialog.suggest.levels);
        assert_eq!(dialog.rows(), Rows::Variants(vec![0]));
        dialog.suggest.levels = true;
        assert_eq!(dialog.rows(), Rows::Variants(vec![0, 1]));
    }

    #[test]
    fn a_failure_is_reported_once_and_the_list_has_only_the_last_row() {
        let (mut dialog, fake) = open(&linked_game());
        fake.answer("g", Err("offline".to_owned()));
        dialog.tick();
        assert!(!dialog.awaiting);
        dialog.start_adding();
        type_in(&mut dialog, "Low%");
        assert_eq!(dialog.rows(), Rows::Variants(vec![]));
        assert_eq!(dialog.row_count(), 1);
        assert_eq!(dialog.take_failure(), Some("offline".to_owned()));
        assert_eq!(dialog.take_failure(), None);
        fake.answer("g", Err("again".to_owned()));
        dialog.tick();
        assert_eq!(dialog.take_failure(), None);
    }

    /// Runs one frame of the dialog with `events`.
    fn frame(
        ctx: &Context,
        dialog: &mut CategoryDialog,
        events: Vec<Event>,
    ) -> Option<CategoryDialogAction> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 800.0))),
            events,
            ..RawInput::default()
        };
        let mut action = None;
        let mut output = ctx.run_ui(input, |ui| action = category_dialog(ui.ctx(), dialog));
        output.textures_delta.clear();
        action
    }

    fn press(key: Key) -> Vec<Event> {
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]
    }

    /// A linked game's dialog drawn in a context, adding a category, with
    /// its variants in and the list open.
    fn adding_on_screen() -> (Context, CategoryDialog) {
        let ctx = Context::default();
        crate::ui::theme::install(&ctx);
        let (mut dialog, fake) = open(&linked_game());
        arrive(&mut dialog, &fake, vec![pc(), n64()]);
        dialog.start_adding();
        assert_eq!(frame(&ctx, &mut dialog, vec![]), None);
        assert_eq!(frame(&ctx, &mut dialog, vec![]), None);
        assert!(dialog.suggest.list_open);
        (ctx, dialog)
    }

    #[test]
    fn enter_on_a_highlighted_row_picks_it_without_adding() {
        let (ctx, mut dialog) = adding_on_screen();
        assert_eq!(frame(&ctx, &mut dialog, press(Key::ArrowDown)), None);
        assert_eq!(dialog.suggest.highlight, Some(0));
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Enter)), None);
        assert_eq!(name(&dialog), "Any% (PC)");
        assert_eq!(dialog.suggest.link, Some(pc().link));
        assert!(matches!(dialog.mode, Mode::Adding { .. }));
    }

    #[test]
    fn escape_closes_the_list_before_leaving_the_edit() {
        let (ctx, mut dialog) = adding_on_screen();
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Escape)), None);
        assert!(!dialog.suggest.list_open);
        assert!(matches!(dialog.mode, Mode::Adding { .. }));
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Escape)), None);
        assert_eq!(dialog.mode, Mode::List);
        assert_eq!(
            frame(&ctx, &mut dialog, press(Key::Escape)),
            Some(CategoryDialogAction::Cancel)
        );
    }
}
