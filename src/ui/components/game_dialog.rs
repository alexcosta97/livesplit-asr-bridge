//! Game dialog: "Which game is this auto splitter for?" for a new `.wasm`,
//! and "Change game" from the Auto splitter card (spec §6.8).

use eframe::egui::{
    self, Align, Frame, Id, Key, Label, Margin, Modal, RichText, ScrollArea, Sense, Stroke,
    StrokeKind, TextEdit, Ui, vec2,
};

use super::{BACKDROP, button::Button, section_label::section_label};
use crate::{
    config::{GameSummary, name_from_file},
    ui::theme,
};

/// The dialog's width.
const WIDTH: f32 = 400.0;
/// The height of the name field, as tall as a button.
const FIELD_HEIGHT: f32 = 32.0;
/// The height of a row in the list of games.
const ROW_HEIGHT: f32 = 32.0;
/// The list of games scrolls beyond this height.
const LIST_HEIGHT: f32 = 200.0;

/// Why the dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameDialogKind {
    /// A `.wasm` with no game yet is being opened.
    New,
    /// **Change** was pressed for the loaded auto splitter.
    Change,
}

/// The game picked in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameChoice {
    /// A game already set up, by slug.
    Existing(String),
    /// A new game, by name.
    New(String),
}

/// What was done in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameDialogAction {
    Use(GameChoice),
    Cancel,
}

/// The Game dialog's state: the name typed and the game selected.
#[derive(Debug, Clone)]
pub struct GameDialog {
    kind: GameDialogKind,
    file_name: String,
    games: Vec<GameSummary>,
    name: String,
    /// The slug of the selected game. It follows the name: typing the name
    /// of a game already set up selects it.
    selected: Option<String>,
    focus_pending: bool,
}

impl GameDialog {
    /// The dialog for `file_name`, a new auto splitter: the name is filled in
    /// from the file name.
    pub fn new(file_name: &str, games: Vec<GameSummary>) -> Self {
        let mut dialog = Self {
            kind: GameDialogKind::New,
            file_name: file_name.to_owned(),
            games,
            name: name_from_file(file_name),
            selected: None,
            focus_pending: true,
        };
        dialog.follow_name();
        dialog
    }

    /// The "Change game" dialog for `file_name`, with its game `current`
    /// selected.
    pub fn change(file_name: &str, games: Vec<GameSummary>, current: &GameSummary) -> Self {
        Self {
            kind: GameDialogKind::Change,
            file_name: file_name.to_owned(),
            games,
            name: current.name.clone(),
            selected: Some(current.slug.clone()),
            focus_pending: true,
        }
    }

    pub fn kind(&self) -> GameDialogKind {
        self.kind
    }

    /// The game **Use this game** would pick, or `None` with no name.
    pub fn choice(&self) -> Option<GameChoice> {
        if let Some(slug) = &self.selected {
            return Some(GameChoice::Existing(slug.clone()));
        }
        let name = self.name.trim();
        (!name.is_empty()).then(|| GameChoice::New(name.to_owned()))
    }

    /// Selects a game from the list, which also fills in its name.
    fn select(&mut self, index: usize) {
        let game = &self.games[index];
        self.name = game.name.clone();
        self.selected = Some(game.slug.clone());
    }

    /// Selects the game whose name is the one typed, ignoring case and
    /// surrounding spaces, or none.
    fn follow_name(&mut self) {
        let name = self.name.trim().to_lowercase();
        self.selected = self
            .games
            .iter()
            .find(|game| game.name.trim().to_lowercase() == name)
            .map(|game| game.slug.clone());
    }

    fn title(&self) -> &'static str {
        match self.kind {
            GameDialogKind::New => "Which game is this auto splitter for?",
            GameDialogKind::Change => "Change game",
        }
    }
}

/// Shows the Game dialog over the window. Escape and clicking outside it
/// cancel it.
pub fn game_dialog(ctx: &egui::Context, dialog: &mut GameDialog) -> Option<GameDialogAction> {
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("game dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, dialog));
    if response.should_close() {
        return Some(GameDialogAction::Cancel);
    }
    response.inner
}

fn contents(ui: &mut Ui, dialog: &mut GameDialog) -> Option<GameDialogAction> {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.add(
        Label::new(
            RichText::new(dialog.title().to_uppercase())
                .font(theme::display(16.0))
                .color(theme::TEXT),
        )
        .wrap(),
    );
    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new(&dialog.file_name)
                .font(theme::mono(12.0))
                .color(theme::TEXT_SECONDARY),
        )
        .truncate(),
    );

    ui.add_space(16.0);
    section_label(ui, "Game");
    ui.add_space(6.0);
    let field = ui.add(
        TextEdit::singleline(&mut dialog.name)
            .font(theme::body(14.0))
            .hint_text("Game name")
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, FIELD_HEIGHT))
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(10, 0)),
    );
    if dialog.focus_pending {
        field.request_focus();
        dialog.focus_pending = false;
    }
    if field.changed() {
        dialog.follow_name();
    }
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));

    if !dialog.games.is_empty() {
        ui.add_space(16.0);
        section_label(ui, "Games already set up");
        ui.add_space(6.0);
        ScrollArea::vertical()
            .max_height(LIST_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let mut clicked = None;
                for (index, game) in dialog.games.iter().enumerate() {
                    let selected = dialog.selected.as_deref() == Some(game.slug.as_str());
                    if game_row(ui, game, selected) {
                        clicked = Some(index);
                    }
                }
                if let Some(index) = clicked {
                    dialog.select(index);
                }
            });
    }

    ui.add_space(24.0);
    let choice = dialog.choice();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let use_game = ui.add(Button::primary("Use this game").enabled(choice.is_some()));
        let cancel = ui.add(Button::secondary("Cancel"));
        if use_game.clicked() || entered {
            choice.map(GameDialogAction::Use)
        } else if cancel.clicked() {
            Some(GameDialogAction::Cancel)
        } else {
            None
        }
    })
    .inner
}

/// A game in the list: its name, and how many auto splitters use it.
/// Returns whether it was clicked.
fn game_row(ui: &mut Ui, game: &GameSummary, selected: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if selected || response.hovered() {
            let stroke = if selected {
                Stroke::new(1.0, theme::ACCENT)
            } else {
                Stroke::NONE
            };
            painter.rect(
                rect,
                theme::RADIUS,
                theme::RAISED,
                stroke,
                StrokeKind::Inside,
            );
        }
        let count = match game.splitters {
            1 => "1 auto splitter".to_owned(),
            n => format!("{n} auto splitters"),
        };
        let count = painter.layout_no_wrap(count, theme::mono(11.0), theme::TEXT_MUTED);
        let count_pos = rect.right_center() - vec2(12.0 + count.size().x, count.size().y / 2.0);
        let name_width = (count_pos.x - rect.left() - 24.0).max(0.0);
        let name = painter.layout(
            game.name.clone(),
            theme::body(13.0),
            theme::TEXT,
            name_width,
        );
        let name_pos = rect.left_center() + vec2(12.0, -name.size().y / 2.0);
        painter
            .with_clip_rect(rect.intersect(ui.clip_rect()))
            .galley(name_pos, name, theme::TEXT);
        painter.galley(count_pos, count, theme::TEXT_MUTED);
    }
    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(slug: &str, name: &str, splitters: usize) -> GameSummary {
        GameSummary {
            slug: slug.to_owned(),
            name: name.to_owned(),
            splitters,
        }
    }

    fn games() -> Vec<GameSummary> {
        vec![
            game("gta-sa", "GTA San Andreas", 2),
            game("portal", "Portal", 1),
        ]
    }

    #[test]
    fn a_new_file_suggests_a_new_game_named_after_it() {
        let dialog = GameDialog::new("celeste_autosplitter.wasm", games());
        assert_eq!(dialog.kind(), GameDialogKind::New);
        assert_eq!(dialog.choice(), Some(GameChoice::New("celeste".to_owned())));
    }

    #[test]
    fn a_file_named_like_a_game_already_set_up_selects_it() {
        let dialog = GameDialog::new("portal.wasm", games());
        assert_eq!(
            dialog.choice(),
            Some(GameChoice::Existing("portal".to_owned()))
        );
    }

    #[test]
    fn selecting_a_game_fills_in_its_name() {
        let mut dialog = GameDialog::new("x.wasm", games());
        dialog.select(0);
        assert_eq!(dialog.name, "GTA San Andreas");
        assert_eq!(
            dialog.choice(),
            Some(GameChoice::Existing("gta-sa".to_owned()))
        );
    }

    #[test]
    fn typing_another_name_picks_a_new_game() {
        let mut dialog = GameDialog::new("x.wasm", games());
        dialog.select(0);
        dialog.name = "GTA Vice City ".to_owned();
        dialog.follow_name();
        assert_eq!(
            dialog.choice(),
            Some(GameChoice::New("GTA Vice City".to_owned()))
        );
    }

    #[test]
    fn typing_a_known_name_in_any_case_selects_that_game() {
        let mut dialog = GameDialog::new("x.wasm", games());
        dialog.name = " gta san andreas".to_owned();
        dialog.follow_name();
        assert_eq!(
            dialog.choice(),
            Some(GameChoice::Existing("gta-sa".to_owned()))
        );
    }

    #[test]
    fn a_blank_name_picks_nothing() {
        let mut dialog = GameDialog::new("x.wasm", games());
        dialog.name = "   ".to_owned();
        dialog.follow_name();
        assert_eq!(dialog.choice(), None);
    }

    #[test]
    fn changing_starts_with_the_current_game_selected() {
        let games = games();
        let dialog = GameDialog::change("gta.wasm", games.clone(), &games[1]);
        assert_eq!(dialog.kind(), GameDialogKind::Change);
        assert_eq!(dialog.title(), "Change game");
        assert_eq!(dialog.name, "Portal");
        assert_eq!(
            dialog.choice(),
            Some(GameChoice::Existing("portal".to_owned()))
        );
    }
}
