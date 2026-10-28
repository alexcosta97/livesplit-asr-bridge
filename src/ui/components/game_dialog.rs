//! Game dialog: "Which game is this auto splitter for?" for a new `.wasm`,
//! and "Change game" from the Auto splitter card (spec §6.8). Its name field
//! also suggests games from speedrun.com and links the game to one (spec
//! §7.5).

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use eframe::egui::{
    self, Align, Area, Color32, FontId, Frame, Galley, Id, Key, Label, Margin, Modal, Modifiers,
    Order, Rect, RichText, ScrollArea, Sense, Shadow, Stroke, StrokeKind, TextEdit, Ui,
    text::{LayoutJob, TextWrapping},
    vec2,
};

use super::{
    BACKDROP,
    button::{Button, ButtonSize},
    section_label::section_label,
};
use crate::{
    config::{GameSummary, name_from_file},
    speedrun::{GameSearch, SpeedrunGame, SpeedrunLink, clear_match},
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
/// The height of a row in the list of speedrun.com results.
pub(super) const RESULT_HEIGHT: f32 = 32.0;
/// How many results the list shows before it scrolls.
pub(super) const RESULTS_SHOWN: f32 = 5.0;
/// How long after typing stops the name is searched, in seconds.
const TYPING_PAUSE: f64 = 0.3;
/// How often the dialog looks for answers while it waits for them.
pub(super) const POLL_EVERY: Duration = Duration::from_millis(100);

/// Why the dialog is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameDialogKind {
    /// A `.wasm` with no game yet is being opened.
    New,
    /// **Change** was pressed for the loaded auto splitter.
    Change,
}

/// The game the dialog's choice is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTarget {
    /// A game already set up, by slug.
    Existing(String),
    /// A new game, by name.
    New(String),
}

/// The game picked in the dialog, and the speedrun.com game it is linked
/// to, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameChoice {
    pub game: GameTarget,
    pub link: Option<SpeedrunLink>,
}

/// What was done in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameDialogAction {
    Use(GameChoice),
    Cancel,
}

/// The Game dialog's state: the name typed, the game selected, and the
/// speedrun.com search and link.
pub struct GameDialog {
    kind: GameDialogKind,
    file_name: String,
    games: Vec<GameSummary>,
    name: String,
    /// The slug of the selected game. It follows the name: typing the name
    /// of a game already set up selects it.
    selected: Option<String>,
    focus_pending: bool,
    /// The speedrun.com game the name is linked to.
    link: Option<SpeedrunLink>,
    /// Looks names up on speedrun.com.
    search: Box<dyn GameSearch>,
    /// The results of each name searched so far, by [`key`] of the name.
    results: HashMap<String, Vec<SpeedrunGame>>,
    /// The name to search next, and the time it is sent at.
    due: Option<(String, f64)>,
    /// The last name sent, until its answer comes back.
    awaiting: Option<String>,
    /// Whether the list of results is showing.
    list_open: bool,
    /// The highlighted row of the list; the last row's index is the number
    /// of results.
    highlight: Option<usize>,
    /// Whether the highlight moved with the keyboard and should be scrolled
    /// to.
    highlight_moved: bool,
    /// Whether the name in the field wasn't typed, so a clear match links it
    /// without asking.
    may_link_on_own: bool,
    /// Names kept unlinked with the list's last row, by [`key`]: they aren't
    /// linked on their own again, but still open the list.
    declined: HashSet<String>,
    /// The first search error, until it is taken.
    failure: Option<String>,
    /// Whether the search error was taken, so it is reported only once.
    failure_taken: bool,
}

impl fmt::Debug for GameDialog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GameDialog")
            .field("kind", &self.kind)
            .field("file_name", &self.file_name)
            .field("name", &self.name)
            .field("selected", &self.selected)
            .field("link", &self.link)
            .field("list_open", &self.list_open)
            .finish_non_exhaustive()
    }
}

impl GameDialog {
    /// The dialog for `file_name`, a new auto splitter: the name is filled in
    /// from the file name, and searched on speedrun.com with `search`.
    pub fn new(file_name: &str, games: Vec<GameSummary>, search: Box<dyn GameSearch>) -> Self {
        let mut dialog = Self::open(
            GameDialogKind::New,
            file_name,
            games,
            name_from_file(file_name),
            search,
        );
        dialog.follow_name();
        dialog.link = dialog.selected_link();
        dialog.search_unless_linked();
        dialog
    }

    /// The "Change game" dialog for `file_name`, with its game `current`
    /// selected. Unless `current` is linked, its name is searched with
    /// `search`.
    pub fn change(
        file_name: &str,
        games: Vec<GameSummary>,
        current: &GameSummary,
        search: Box<dyn GameSearch>,
    ) -> Self {
        let mut dialog = Self::open(
            GameDialogKind::Change,
            file_name,
            games,
            current.name.clone(),
            search,
        );
        dialog.selected = Some(current.slug.clone());
        dialog.link = link_of(current);
        dialog.search_unless_linked();
        dialog
    }

    /// A dialog with `name` in the field, nothing selected and nothing
    /// searched yet.
    fn open(
        kind: GameDialogKind,
        file_name: &str,
        games: Vec<GameSummary>,
        name: String,
        search: Box<dyn GameSearch>,
    ) -> Self {
        Self {
            kind,
            file_name: file_name.to_owned(),
            games,
            name,
            selected: None,
            focus_pending: true,
            link: None,
            search,
            results: HashMap::new(),
            due: None,
            awaiting: None,
            list_open: false,
            highlight: None,
            highlight_moved: false,
            may_link_on_own: false,
            declined: HashSet::new(),
            failure: None,
            failure_taken: false,
        }
    }

    pub fn kind(&self) -> GameDialogKind {
        self.kind
    }

    /// The game **Use this game** would pick, or `None` with no name. A link
    /// to a speedrun.com game that a game already set up is linked to picks
    /// that game.
    pub fn choice(&self) -> Option<GameChoice> {
        let linked_game = self.link.as_ref().and_then(|link| {
            self.games
                .iter()
                .find(|game| game.speedrun_id.as_deref() == Some(link.id.as_str()))
        });
        let game = if let Some(game) = linked_game {
            GameTarget::Existing(game.slug.clone())
        } else if let Some(slug) = &self.selected {
            GameTarget::Existing(slug.clone())
        } else {
            let name = self.name.trim();
            if name.is_empty() {
                return None;
            }
            GameTarget::New(name.to_owned())
        };
        Some(GameChoice {
            game,
            link: self.link.clone(),
        })
    }

    /// The error of the first search that failed, once per dialog.
    pub fn take_failure(&mut self) -> Option<String> {
        let failure = self.failure.take();
        self.failure_taken |= failure.is_some();
        failure
    }

    /// Selects a game from the list, which also fills in its name and brings
    /// its link. A game that isn't linked is searched.
    fn select(&mut self, index: usize) {
        let game = &self.games[index];
        self.name = game.name.clone();
        self.selected = Some(game.slug.clone());
        self.link = link_of(game);
        self.close_list();
        self.due = None;
        self.search_unless_linked();
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

    /// The link of the selected game, if it has one.
    fn selected_link(&self) -> Option<SpeedrunLink> {
        let slug = self.selected.as_ref()?;
        self.games
            .iter()
            .find(|game| &game.slug == slug)
            .and_then(link_of)
    }

    /// The name in the field changed at `now`: the link goes, a game already
    /// set up with that name brings its own, and otherwise the name is
    /// searched once typing pauses. A typed name is never linked on its own.
    fn typed(&mut self, now: f64) {
        self.follow_name();
        self.link = self.selected_link();
        self.may_link_on_own = false;
        self.close_list();
        let name = self.name.trim();
        self.due = (self.link.is_none() && !name.is_empty())
            .then(|| (name.to_owned(), now + TYPING_PAUSE));
    }

    /// Searches the name in the field on the next tick, linking it on its
    /// own if one result clearly matches.
    fn search_now(&mut self) {
        self.may_link_on_own = true;
        let name = self.name.trim();
        self.due = (!name.is_empty()).then(|| (name.to_owned(), f64::NEG_INFINITY));
    }

    /// Searches the name in the field, unless it is linked.
    fn search_unless_linked(&mut self) {
        if self.link.is_none() {
            self.search_now();
        }
    }

    /// Runs every frame, at `now`: sends the search that is due, and takes
    /// in the answers that came back.
    fn tick(&mut self, now: f64) {
        if let Some((name, at)) = self.due.take() {
            if now < at {
                self.due = Some((name, at));
            } else if self.results.contains_key(&key(&name)) {
                self.arrived(&name, false);
            } else {
                self.search.request(&name);
                self.awaiting = Some(name);
            }
        }
        while let Some(answer) = self.search.poll() {
            if self.awaiting.as_ref() == Some(&answer.name) {
                self.awaiting = None;
            }
            let failed = match answer.games {
                Ok(games) => {
                    self.results.insert(key(&answer.name), games);
                    false
                }
                Err(error) => {
                    if self.failure.is_none() && !self.failure_taken {
                        self.failure = Some(error);
                    }
                    true
                }
            };
            self.arrived(&answer.name, failed);
        }
    }

    /// Results, or a failure when `failed`, are in for `name`. For the name
    /// in the field, unless it is linked, a clear match links on its own when
    /// it may and the name wasn't declined, and otherwise the list opens.
    fn arrived(&mut self, name: &str, failed: bool) {
        let name_key = key(name);
        if name_key != key(&self.name) || self.link.is_some() {
            return;
        }
        let games = self.shown();
        let clear = (self.may_link_on_own && !failed && !self.declined.contains(&name_key))
            .then(|| clear_match(name, games))
            .flatten();
        match clear {
            Some(index) => self.pick(index),
            None => {
                self.list_open = true;
                self.highlight = None;
            }
        }
    }

    /// The results for the name in the field: none until they arrive, and
    /// none when its search failed.
    fn shown(&self) -> &[SpeedrunGame] {
        self.results
            .get(&key(&self.name))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Links the result at `index` and fills the field with its name. The
    /// selected game stays selected; with none selected, a game already set
    /// up under that name is, as typing it would, so it isn't set up twice.
    fn pick(&mut self, index: usize) {
        let game = self.shown()[index].clone();
        self.link = Some(SpeedrunLink::from(&game));
        self.name = game.name;
        if self.selected.is_none() {
            self.follow_name();
        }
        self.due = None;
        self.close_list();
    }

    /// The list's last row: keeps the name typed, unlinked, and doesn't link
    /// it on its own again.
    fn pick_typed(&mut self) {
        self.declined.insert(key(&self.name));
        self.link = None;
        self.close_list();
    }

    /// **Change** on the linked line, at `now`: removes the link and opens
    /// the list for the name in the field, searching it if needed.
    fn change_link(&mut self, now: f64) {
        self.link = None;
        self.may_link_on_own = false;
        let name = self.name.trim().to_owned();
        if self.results.contains_key(&key(&name)) {
            self.arrived(&name, false);
        } else if !name.is_empty() {
            self.due = Some((name, now));
        }
    }

    fn close_list(&mut self) {
        self.list_open = false;
        self.highlight = None;
    }

    /// A key pressed while the list is open: Up and Down move the highlight,
    /// Enter picks it, and Escape closes the list. Returns whether the key
    /// was used, so it can be kept from the field and the dialog.
    fn key(&mut self, key: Key) -> bool {
        if !self.list_open {
            return false;
        }
        let last = self.shown().len();
        match key {
            Key::ArrowDown => {
                self.highlight = Some(self.highlight.map_or(0, |row| (row + 1).min(last)));
                self.highlight_moved = true;
            }
            Key::ArrowUp => {
                self.highlight = Some(self.highlight.map_or(0, |row| row.saturating_sub(1)));
                self.highlight_moved = true;
            }
            Key::Enter => match self.highlight {
                Some(row) if row == last => self.pick_typed(),
                Some(row) => self.pick(row),
                None => return false,
            },
            Key::Escape => self.close_list(),
            _ => return false,
        }
        true
    }

    fn title(&self) -> &'static str {
        match self.kind {
            GameDialogKind::New => "Which game is this auto splitter for?",
            GameDialogKind::Change => "Change game",
        }
    }
}

/// How results and declined names are keyed: the name trimmed and in lower
/// case, so a name differing only in case, like one renamed by a link, shares
/// its results.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// The speedrun.com link of a game already set up, if it has one. A linked
/// game is named as on speedrun.com.
fn link_of(game: &GameSummary) -> Option<SpeedrunLink> {
    game.speedrun_id.as_ref().map(|id| SpeedrunLink {
        id: id.clone(),
        name: game.name.clone(),
    })
}

/// Shows the Game dialog over the window. Escape and clicking outside it
/// cancel it; with the list of results open, Escape closes the list.
pub fn game_dialog(ctx: &egui::Context, dialog: &mut GameDialog) -> Option<GameDialogAction> {
    let now = ctx.input(|input| input.time);
    dialog.tick(now);
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("game dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, dialog, now));
    // After the contents, so a search made due this frame, by typing or
    // Change, is sent without waiting for more input.
    if dialog.due.is_some() || dialog.awaiting.is_some() {
        ctx.request_repaint_after(POLL_EVERY);
    }
    if response.should_close() {
        return Some(GameDialogAction::Cancel);
    }
    response.inner
}

fn contents(ui: &mut Ui, dialog: &mut GameDialog, now: f64) -> Option<GameDialogAction> {
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
    // The list takes its keys before the field and the modal see them, so
    // Escape closes the list rather than the dialog, and an Enter that picks
    // a row isn't also **Use this game**.
    let list_closed = list_keys(ui, dialog);
    let field = ui.add(
        TextEdit::singleline(&mut dialog.name)
            .font(theme::body(14.0))
            .hint_text("Game name")
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, FIELD_HEIGHT))
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(10, 0)),
    );
    if dialog.focus_pending || list_closed {
        field.request_focus();
        dialog.focus_pending = false;
    }
    if field.changed() {
        dialog.typed(now);
    }
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));

    if dialog.link.is_some() && linked_line(ui) {
        dialog.change_link(now);
    }
    if dialog.list_open {
        results_list(ui.ctx(), field.rect, dialog);
    }

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

    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(
                "Settings are saved per game and category, so auto splitters for the same \
                 game share them.",
            )
            .font(theme::body(12.0))
            .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );

    ui.add_space(20.0);
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

/// Feeds the keys pressed this frame to the open list, taking the ones it
/// uses. Returns whether Escape closed the list, which also takes the
/// field's focus, so the field can ask for it back.
fn list_keys(ui: &mut Ui, dialog: &mut GameDialog) -> bool {
    let mut closed = false;
    for key in [Key::ArrowDown, Key::ArrowUp, Key::Enter, Key::Escape] {
        ui.input_mut(|input| {
            if input.modifiers.is_none() && input.key_pressed(key) && dialog.key(key) {
                input.consume_key(Modifiers::NONE, key);
                closed |= key == Key::Escape;
            }
        });
    }
    closed
}

/// The line under the field while the game is linked: a green dot,
/// "Linked to speedrun.com" and **Change**. Returns whether **Change** was
/// clicked.
pub(super) fn linked_line(ui: &mut Ui) -> bool {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 4.0, theme::STATUS_OK);
        ui.label(
            RichText::new("Linked to speedrun.com")
                .font(theme::body(12.0))
                .color(theme::TEXT_SECONDARY),
        );
        ui.add(Button::secondary("Change").size(ButtonSize::Xs))
            .clicked()
    })
    .inner
}

/// A row of the list of results that was clicked.
enum Clicked {
    /// The result at this index.
    Result(usize),
    /// The last row, which keeps the typed name.
    Typed,
}

/// The list of speedrun.com results, dropping down from the field over what
/// is under it: five rows that scroll for more, then the last row, which
/// keeps the typed name. A click outside the list and the field closes it.
fn results_list(ctx: &egui::Context, field: Rect, dialog: &mut GameDialog) {
    let games = dialog.shown().to_vec();
    let typed = format!("Use \"{}\" (not on speedrun.com)", dialog.name.trim());
    let scroll_to = dialog.highlight.filter(|_| dialog.highlight_moved);
    dialog.highlight_moved = false;
    let frame = list_frame();
    let area = Area::new(Id::new("game dialog results"))
        .order(Order::Tooltip)
        .fixed_pos(field.left_bottom())
        .show(ctx, |ui| {
            frame
                .show(ui, |ui| {
                    let width = field.width() - 2.0;
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let mut clicked = None;
                    if !games.is_empty() {
                        ScrollArea::vertical()
                            .max_height(RESULTS_SHOWN * RESULT_HEIGHT)
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (index, game) in games.iter().enumerate() {
                                    let set_up = dialog
                                        .games
                                        .iter()
                                        .any(|g| g.speedrun_id.as_deref() == Some(&game.id));
                                    let highlighted = dialog.highlight == Some(index);
                                    let row = result_row(ui, width, game, set_up, highlighted);
                                    if scroll_to == Some(index) {
                                        row.scroll_to_me(None);
                                    }
                                    if row.clicked() {
                                        clicked = Some(Clicked::Result(index));
                                    }
                                }
                            });
                        let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
                        ui.painter().rect_filled(line, 0.0, theme::BORDER);
                    }
                    let highlighted = dialog.highlight == Some(games.len());
                    if typed_row(ui, width, typed, highlighted) {
                        clicked = Some(Clicked::Typed);
                    }
                    clicked
                })
                .inner
        });
    match area.inner {
        Some(Clicked::Result(index)) => dialog.pick(index),
        Some(Clicked::Typed) => dialog.pick_typed(),
        None => {
            let outside = ctx.input(|input| {
                input.pointer.any_pressed()
                    && input.pointer.interact_pos().is_some_and(|pos| {
                        !area.response.rect.contains(pos) && !field.contains(pos)
                    })
            });
            if outside {
                dialog.close_list();
            }
            return;
        }
    }
    // The click took the field's focus; it gets it back.
    dialog.focus_pending = true;
}

/// The frame of a list dropping down from a field: raised, with a strong
/// border and a shadow.
pub(super) fn list_frame() -> Frame {
    Frame::NONE
        .fill(theme::RAISED)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .shadow(Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(128),
        })
}

/// A result in the list: its speedrun.com name, SET UP when a game already
/// set up is linked to it, and the year it was released.
fn result_row(
    ui: &mut Ui,
    width: f32,
    game: &SpeedrunGame,
    set_up: bool,
    highlighted: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, RESULT_HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if highlighted || response.hovered() {
            painter.rect_filled(rect, 0.0, theme::BORDER);
        }
        let mut right = rect.right() - 12.0;
        let mut tag = |text: String, color: Color32| {
            let galley = painter.layout_no_wrap(text, theme::mono(11.0), color);
            let pos = egui::pos2(
                right - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            right = pos.x - 8.0;
            painter.galley(pos, galley, color);
        };
        if let Some(year) = game.year {
            tag(year.to_string(), theme::TEXT_MUTED);
        }
        if set_up {
            tag("SET UP".to_owned(), theme::TEXT_SECONDARY);
        }
        let name = one_line(
            ui,
            game.name.clone(),
            theme::body(13.0),
            theme::TEXT,
            (right - rect.left() - 16.0).max(0.0),
        );
        let pos = egui::pos2(rect.left() + 12.0, rect.center().y - name.size().y / 2.0);
        ui.painter().galley(pos, name, theme::TEXT);
    }
    response
}

/// The list's last row: `Use "<typed name>" (not on speedrun.com)`. Returns
/// whether it was clicked.
pub(super) fn typed_row(ui: &mut Ui, width: f32, text: String, highlighted: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(width, RESULT_HEIGHT), Sense::click());
    if ui.is_rect_visible(rect) {
        if highlighted || response.hovered() {
            ui.painter().rect_filled(rect, 0.0, theme::BORDER);
        }
        let galley = one_line(
            ui,
            text,
            theme::body(13.0),
            theme::TEXT_SECONDARY,
            width - 24.0,
        );
        let pos = egui::pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, theme::TEXT_SECONDARY);
    }
    response.clicked()
}

/// `text` on one line, cut short with an ellipsis beyond `width`.
pub(super) fn one_line(
    ui: &Ui,
    text: String,
    font: FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(text, font, color);
    job.wrap = TextWrapping::truncate_at_width(width);
    ui.painter().layout_job(job)
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
    use eframe::egui::{Context, Event, Modifiers, Pos2, RawInput, Rect};

    use super::*;
    use crate::speedrun::FakeSearch;

    fn game(slug: &str, name: &str, splitters: usize) -> GameSummary {
        GameSummary {
            slug: slug.to_owned(),
            name: name.to_owned(),
            speedrun_id: None,
            splitters,
        }
    }

    fn linked_game(slug: &str, name: &str, id: &str) -> GameSummary {
        GameSummary {
            speedrun_id: Some(id.to_owned()),
            ..game(slug, name, 1)
        }
    }

    fn games() -> Vec<GameSummary> {
        vec![
            game("gta-sa", "GTA San Andreas", 2),
            game("portal", "Portal", 1),
        ]
    }

    fn result(id: &str, name: &str, abbreviation: &str, year: u16) -> SpeedrunGame {
        SpeedrunGame {
            id: id.to_owned(),
            name: name.to_owned(),
            abbreviation: abbreviation.to_owned(),
            year: Some(year),
        }
    }

    fn portal() -> SpeedrunGame {
        result("4pd0n31e", "Portal", "portal", 2007)
    }

    fn portals() -> SpeedrunGame {
        result("m1zz5210", "Portals", "portals", 2019)
    }

    fn portal_link() -> SpeedrunLink {
        SpeedrunLink::from(&portal())
    }

    /// A new-file dialog searching with a fake, and the fake to drive it.
    fn new_dialog(file_name: &str, games: Vec<GameSummary>) -> (GameDialog, FakeSearch) {
        let fake = FakeSearch::default();
        let dialog = GameDialog::new(file_name, games, Box::new(fake.clone()));
        (dialog, fake)
    }

    fn existing(slug: &str, link: Option<SpeedrunLink>) -> Option<GameChoice> {
        Some(GameChoice {
            game: GameTarget::Existing(slug.to_owned()),
            link,
        })
    }

    fn new_game(name: &str, link: Option<SpeedrunLink>) -> Option<GameChoice> {
        Some(GameChoice {
            game: GameTarget::New(name.to_owned()),
            link,
        })
    }

    /// Types `name` into the field at `now`.
    fn type_name(dialog: &mut GameDialog, name: &str, now: f64) {
        dialog.name = name.to_owned();
        dialog.typed(now);
    }

    #[test]
    fn a_new_file_suggests_a_new_game_named_after_it() {
        let (dialog, _) = new_dialog("celeste_autosplitter.wasm", games());
        assert_eq!(dialog.kind(), GameDialogKind::New);
        assert_eq!(dialog.choice(), new_game("celeste", None));
    }

    #[test]
    fn a_file_named_like_a_game_already_set_up_selects_it() {
        let (dialog, _) = new_dialog("portal.wasm", games());
        assert_eq!(dialog.choice(), existing("portal", None));
    }

    #[test]
    fn selecting_a_game_fills_in_its_name() {
        let (mut dialog, _) = new_dialog("x.wasm", games());
        dialog.select(0);
        assert_eq!(dialog.name, "GTA San Andreas");
        assert_eq!(dialog.choice(), existing("gta-sa", None));
    }

    #[test]
    fn typing_another_name_picks_a_new_game() {
        let (mut dialog, _) = new_dialog("x.wasm", games());
        dialog.select(0);
        type_name(&mut dialog, "GTA Vice City ", 0.0);
        assert_eq!(dialog.choice(), new_game("GTA Vice City", None));
    }

    #[test]
    fn typing_a_known_name_in_any_case_selects_that_game() {
        let (mut dialog, _) = new_dialog("x.wasm", games());
        type_name(&mut dialog, " gta san andreas", 0.0);
        assert_eq!(dialog.choice(), existing("gta-sa", None));
    }

    #[test]
    fn a_blank_name_picks_nothing() {
        let (mut dialog, _) = new_dialog("x.wasm", games());
        type_name(&mut dialog, "   ", 0.0);
        assert_eq!(dialog.choice(), None);
    }

    #[test]
    fn changing_starts_with_the_current_game_selected() {
        let games = games();
        let fake = FakeSearch::default();
        let dialog = GameDialog::change("gta.wasm", games.clone(), &games[1], Box::new(fake));
        assert_eq!(dialog.kind(), GameDialogKind::Change);
        assert_eq!(dialog.title(), "Change game");
        assert_eq!(dialog.name, "Portal");
        assert_eq!(dialog.choice(), existing("portal", None));
    }

    #[test]
    fn opening_searches_the_name_from_the_file_at_once() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        dialog.tick(0.0);
        assert_eq!(fake.requests(), ["portal"]);
    }

    #[test]
    fn a_clear_match_on_open_links_and_renames() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        fake.answer("portal", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        assert_eq!(dialog.name, "Portal");
        assert_eq!(dialog.choice(), new_game("Portal", Some(portal_link())));
        assert!(!dialog.list_open);
    }

    #[test]
    fn no_clear_match_on_open_opens_the_list() {
        let (mut dialog, fake) = new_dialog("gta_sa_de.wasm", vec![]);
        fake.answer("gta sa de", Ok(vec![]));
        dialog.tick(0.0);
        assert!(dialog.list_open);
        assert_eq!(dialog.link, None);
    }

    #[test]
    fn typing_waits_300_ms_and_never_links_on_its_own() {
        let (mut dialog, fake) = new_dialog("x.wasm", vec![]);
        fake.answer("x", Ok(vec![]));
        dialog.tick(0.0);
        type_name(&mut dialog, "portal", 1.0);
        dialog.tick(1.2);
        assert_eq!(fake.requests(), ["x"]);
        dialog.tick(1.31);
        assert_eq!(fake.requests(), ["x", "portal"]);
        fake.answer("portal", Ok(vec![portal()]));
        dialog.tick(1.5);
        assert_eq!(dialog.link, None);
        assert!(dialog.list_open);
        assert_eq!(dialog.name, "portal");
    }

    #[test]
    fn picking_a_result_links_the_selected_game_without_changing_it() {
        let (mut dialog, fake) = new_dialog("x.wasm", vec![game("gta-sa-de", "GTA SA DE", 1)]);
        dialog.select(0);
        let gta = result("yo1yv1q5", "Grand Theft Auto: San Andreas", "gtasa", 2004);
        let remastered = result(
            "j1nevzx1",
            "Grand Theft Auto: San Andreas Remastered",
            "",
            2013,
        );
        fake.answer("GTA SA DE", Ok(vec![gta.clone(), remastered]));
        dialog.tick(0.0);
        assert_eq!(fake.requests(), ["GTA SA DE"]);
        assert!(dialog.list_open, "no result matches clearly");
        assert_eq!(dialog.link, None);
        dialog.pick(0);
        assert_eq!(dialog.name, "Grand Theft Auto: San Andreas");
        assert!(!dialog.list_open);
        assert_eq!(
            dialog.choice(),
            existing("gta-sa-de", Some(SpeedrunLink::from(&gta)))
        );
    }

    fn gta_sa() -> SpeedrunGame {
        result("yo1yv1q5", "Grand Theft Auto: San Andreas", "gtasa", 2004)
    }

    fn gta_sa_remastered() -> SpeedrunGame {
        result(
            "j1nevzx1",
            "Grand Theft Auto: San Andreas Remastered",
            "",
            2013,
        )
    }

    #[test]
    fn picking_a_result_named_like_a_game_set_up_selects_that_game() {
        let set_up = vec![game("gta-sa", "Grand Theft Auto: San Andreas", 2)];
        let (mut dialog, fake) = new_dialog("gta_sa_de.wasm", set_up);
        fake.answer("gta sa de", Ok(vec![gta_sa(), gta_sa_remastered()]));
        dialog.tick(0.0);
        assert!(dialog.list_open, "no result matches clearly");
        dialog.pick(0);
        assert_eq!(
            dialog.choice(),
            existing("gta-sa", Some(SpeedrunLink::from(&gta_sa())))
        );
    }

    #[test]
    fn linking_on_its_own_to_a_name_set_up_selects_that_game() {
        let set_up = vec![game("gta-sa", "Grand Theft Auto: San Andreas", 2)];
        let (mut dialog, fake) = new_dialog("gta_sa.wasm", set_up);
        fake.answer("gta sa", Ok(vec![gta_sa(), gta_sa_remastered()]));
        dialog.tick(0.0);
        assert_eq!(dialog.name, "Grand Theft Auto: San Andreas");
        assert_eq!(
            dialog.choice(),
            existing("gta-sa", Some(SpeedrunLink::from(&gta_sa())))
        );
    }

    #[test]
    fn a_result_already_linked_uses_that_game() {
        let (mut dialog, fake) = new_dialog(
            "portal_2007.wasm",
            vec![linked_game("portal", "Portal", "4pd0n31e")],
        );
        fake.answer("portal 2007", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        assert!(dialog.list_open);
        dialog.pick(0);
        assert_eq!(dialog.choice(), existing("portal", Some(portal_link())));
    }

    #[test]
    fn typing_removes_the_link_and_selecting_a_linked_game_brings_its_link() {
        let (mut dialog, fake) = new_dialog(
            "x.wasm",
            vec![
                linked_game("portal", "Portal", "4pd0n31e"),
                game("gta-sa", "GTA", 1),
            ],
        );
        dialog.select(0);
        assert_eq!(dialog.link, Some(portal_link()));
        type_name(&mut dialog, "Portal 2", 1.0);
        assert_eq!(dialog.link, None);
        type_name(&mut dialog, "portal", 2.0);
        assert_eq!(dialog.link, Some(portal_link()));
        assert_eq!(dialog.choice(), existing("portal", Some(portal_link())));
        dialog.tick(3.0);
        assert!(!fake.requests().contains(&"portal".to_owned()));
    }

    #[test]
    fn the_last_row_keeps_the_typed_name_unlinked_and_stops_linking_it() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        fake.answer("portal", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        dialog.change_link(1.0);
        assert!(dialog.list_open);
        dialog.pick_typed();
        assert!(!dialog.list_open);
        assert_eq!(dialog.link, None);
        assert_eq!(dialog.choice(), new_game("Portal", None));
        dialog.search_now();
        dialog.tick(2.0);
        assert_eq!(
            dialog.link, None,
            "a clear match doesn't link it on its own"
        );
        assert!(dialog.list_open);
    }

    #[test]
    fn a_declined_name_typed_again_opens_the_list_without_linking() {
        let (mut dialog, fake) = new_dialog("portal_x.wasm", vec![]);
        fake.answer("portal x", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        dialog.pick_typed();
        type_name(&mut dialog, "portal 2", 1.0);
        dialog.tick(1.3);
        type_name(&mut dialog, "Portal X", 2.0);
        dialog.tick(2.3);
        assert!(dialog.list_open);
        assert_eq!(dialog.shown().len(), 2);
        assert_eq!(dialog.link, None);
        assert_eq!(fake.requests(), ["portal x", "portal 2"]);
    }

    #[test]
    fn change_unlinks_and_opens_the_list_without_linking_on_its_own() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        fake.answer("portal", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        assert_eq!(dialog.link, Some(portal_link()));
        dialog.change_link(1.0);
        assert_eq!(dialog.link, None);
        dialog.tick(1.0);
        assert_eq!(fake.requests(), ["portal"], "\"Portal\" reuses the results");
        assert!(dialog.list_open);
        assert_eq!(dialog.link, None);
    }

    #[test]
    fn change_game_on_a_linked_game_does_not_search() {
        let current = linked_game("portal", "Portal", "4pd0n31e");
        let fake = FakeSearch::default();
        let mut dialog = GameDialog::change(
            "p.wasm",
            vec![current.clone()],
            &current,
            Box::new(fake.clone()),
        );
        dialog.tick(0.0);
        assert!(fake.requests().is_empty());
        assert_eq!(dialog.link, Some(portal_link()));
    }

    #[test]
    fn answers_for_an_old_name_are_ignored() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        dialog.tick(0.0);
        type_name(&mut dialog, "portal 2", 1.0);
        fake.answer("portal", Ok(vec![portal()]));
        dialog.tick(1.1);
        assert!(!dialog.list_open);
        assert_eq!(dialog.link, None);
    }

    #[test]
    fn results_are_reused_for_a_name_searched_before() {
        let (mut dialog, fake) = new_dialog("x.wasm", vec![]);
        dialog.tick(0.0);
        type_name(&mut dialog, "portal", 1.0);
        dialog.tick(1.3);
        fake.answer("portal", Ok(vec![portal(), portals()]));
        dialog.tick(1.4);
        type_name(&mut dialog, "portal 2", 2.0);
        dialog.tick(2.3);
        type_name(&mut dialog, "portal", 3.0);
        dialog.tick(3.3);
        assert_eq!(fake.requests(), ["x", "portal", "portal 2"]);
        assert!(dialog.list_open);
        assert_eq!(dialog.shown().len(), 2);
    }

    #[test]
    fn offline_shows_only_the_last_row_and_reports_once() {
        let (mut dialog, fake) = new_dialog("portal.wasm", vec![]);
        fake.answer("portal", Err("offline".to_owned()));
        dialog.tick(0.0);
        assert!(dialog.list_open);
        assert!(dialog.shown().is_empty());
        type_name(&mut dialog, "portal 2", 1.0);
        dialog.tick(1.3);
        fake.answer("portal 2", Err("still offline".to_owned()));
        dialog.tick(1.4);
        assert!(dialog.list_open);
        assert!(dialog.shown().is_empty());
        assert_eq!(dialog.take_failure(), Some("offline".to_owned()));
        assert_eq!(dialog.take_failure(), None);
    }

    #[test]
    fn arrows_and_enter_pick_and_escape_closes_the_list() {
        let (mut dialog, fake) = new_dialog("portal_x.wasm", vec![]);
        fake.answer("portal x", Ok(vec![portal(), portals()]));
        dialog.tick(0.0);
        assert!(dialog.list_open);
        assert!(dialog.key(Key::ArrowDown));
        assert_eq!(dialog.highlight, Some(0));
        assert!(dialog.key(Key::ArrowDown));
        assert!(dialog.key(Key::ArrowDown));
        assert!(dialog.key(Key::ArrowDown));
        assert_eq!(dialog.highlight, Some(2), "the last row is the last stop");
        assert!(dialog.key(Key::ArrowUp));
        assert!(dialog.key(Key::ArrowUp));
        assert!(dialog.key(Key::Enter));
        assert_eq!(dialog.link, Some(portal_link()));
        assert!(!dialog.list_open);
        assert!(!dialog.key(Key::Enter));

        dialog.change_link(1.0);
        dialog.tick(1.0);
        fake.answer("Portal", Ok(vec![portal()]));
        dialog.tick(1.1);
        assert!(dialog.list_open);
        assert!(!dialog.key(Key::Enter), "Enter without a highlight is Use");
        assert!(dialog.key(Key::Escape));
        assert!(!dialog.list_open);
        assert!(!dialog.key(Key::Escape));
    }

    /// Runs one frame of the dialog with `events`.
    fn frame(
        ctx: &Context,
        dialog: &mut GameDialog,
        events: Vec<Event>,
    ) -> Option<GameDialogAction> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 800.0))),
            events,
            ..RawInput::default()
        };
        let mut action = None;
        let mut output = ctx.run_ui(input, |ui| action = game_dialog(ui.ctx(), dialog));
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

    /// A dialog drawn in a context, with its list of results open.
    fn open_list_on_screen() -> (Context, GameDialog) {
        let ctx = Context::default();
        crate::ui::theme::install(&ctx);
        let (mut dialog, fake) = new_dialog("portal_x.wasm", vec![]);
        fake.answer("portal x", Ok(vec![portal(), portals()]));
        assert_eq!(frame(&ctx, &mut dialog, vec![]), None);
        assert_eq!(frame(&ctx, &mut dialog, vec![]), None);
        assert!(dialog.list_open);
        (ctx, dialog)
    }

    #[test]
    fn enter_on_a_highlighted_row_picks_it_and_does_not_use_the_game() {
        let (ctx, mut dialog) = open_list_on_screen();
        assert_eq!(frame(&ctx, &mut dialog, press(Key::ArrowDown)), None);
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Enter)), None);
        assert_eq!(dialog.link, Some(portal_link()));
        assert_eq!(dialog.name, "Portal");
    }

    #[test]
    fn escape_closes_the_list_and_a_second_escape_cancels() {
        let (ctx, mut dialog) = open_list_on_screen();
        assert_eq!(frame(&ctx, &mut dialog, press(Key::Escape)), None);
        assert!(!dialog.list_open);
        assert_eq!(
            frame(&ctx, &mut dialog, press(Key::Escape)),
            Some(GameDialogAction::Cancel)
        );
    }
}
