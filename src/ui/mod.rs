//! The application window: the status column and, when wide, the tabs.

mod components;
mod settings;
mod settings_tab;
mod status;
mod theme;

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, mpsc::Receiver},
    time::Instant,
};

use eframe::egui::{self, Frame, Label, Margin, RichText, ScrollArea, ViewportCommand};
use livesplit_auto_splitting::{settings::WidgetKind, wasi_path};
use toml::Value;

use crate::{
    config::{self, AppSettings, Config, Game, GameSummary, Splitters},
    runner::{NoTimer, Runner, RunnerEvent, file_name},
    version::VERSION,
};
use components::{
    ErrorAction, GameChoice, GameDialog, GameDialogAction, GameDialogKind, SplitterAction, Tab,
    UnsavedAction, UnsavedTrigger, Width,
};
use settings::{Setting, SettingsEditor};
use settings_tab::SettingsAction;
use status::Status;

pub use theme::install as install_theme;

/// The application name, used for the window and, later, the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The name shown to people, in the window title.
pub const DISPLAY_NAME: &str = "LiveSplit One ASR Bridge";

/// How many recent Runner messages the window keeps, until the Log tab
/// replaces this list.
const RECENT_MESSAGES: usize = 500;

/// Below this window width, the status column fills the window.
const COMPACT_BELOW: f32 = 640.0;
/// The width of the status column, when wide.
const COLUMN_WIDTH: f32 = 300.0;
/// The space between cards in the status column.
const CARD_GAP: f32 = 12.0;

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{DISPLAY_NAME} {VERSION}")
}

/// The tabs of the tab area. Later issues add Connection and Preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabId {
    Settings,
    /// The Runner's recent messages, until the Log tab (#12) replaces them.
    Log,
}

/// What waits for an answer in the Unsaved dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Reload,
    Open(PathBuf),
    Close,
}

impl Pending {
    fn trigger(&self) -> UnsavedTrigger {
        match self {
            Self::Reload => UnsavedTrigger::Reload,
            Self::Open(_) => UnsavedTrigger::Open,
            Self::Close => UnsavedTrigger::Close,
        }
    }
}

/// The application. Later issues add the Timer and Last action cards and the
/// other tabs.
pub struct BridgeApp {
    ctx: egui::Context,
    runner: Runner,
    events: Receiver<RunnerEvent>,
    status: Status,
    /// Each message, and whether it is an error.
    recent: VecDeque<(String, bool)>,
    /// The configuration folder, if the OS has one for the user.
    config: Option<Config>,
    /// The app's own settings, read from `app.toml` at start-up. The
    /// server (#9) listens on its port.
    app_settings: AppSettings,
    /// The game of each auto splitter being loaded, until it is loaded.
    loading: HashMap<PathBuf, Game>,
    /// The loaded auto splitter's game.
    game: Option<Game>,
    /// The open Game dialog, and the file it is for.
    dialog: Option<(GameDialog, PathBuf)>,
    /// The loaded auto splitter's settings, and the edits not saved yet.
    settings: SettingsEditor,
    tab: TabId,
    /// What the open Unsaved dialog is asking about.
    pending: Option<Pending>,
    /// Closing was asked for and the unsaved settings dealt with.
    closing: bool,
}

impl BridgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        Self::with_config(ctx, Config::standard())
    }

    /// The application, with its files in `config`.
    fn with_config(ctx: &egui::Context, config: Option<Config>) -> Self {
        let repaint = ctx.clone();
        // The Server replaces NoTimer (#10, #11).
        let (runner, events) = Runner::new(Arc::new(NoTimer), move || repaint.request_repaint());
        let mut app = Self {
            ctx: ctx.clone(),
            runner,
            events,
            status: Status::default(),
            recent: VecDeque::new(),
            config,
            app_settings: AppSettings::default(),
            loading: HashMap::new(),
            game: None,
            dialog: None,
            settings: SettingsEditor::default(),
            tab: TabId::Settings,
            pending: None,
            closing: false,
        };
        app.app_settings = app.read_app_settings();
        app
    }

    /// Reads `app.toml`. If it can't be read, the error is reported and the
    /// defaults are used.
    fn read_app_settings(&mut self) -> AppSettings {
        let Some(config) = self.config() else {
            return AppSettings::default();
        };
        AppSettings::load(&config).unwrap_or_else(|error| {
            self.note(format!("Couldn't read the app's settings: {error}"), true);
            AppSettings::default()
        })
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.status.apply(&event);
            match &event {
                RunnerEvent::Loaded { path, .. } => {
                    self.game = self.loading.remove(path);
                    let saved = self.game.as_ref().map(|game| game.settings.clone());
                    self.settings.reset(saved.unwrap_or_default());
                }
                RunnerEvent::LoadFailed { path, .. } => {
                    self.loading.remove(path);
                }
                RunnerEvent::Unloaded => {
                    self.game = None;
                    self.settings.reset(toml::Table::new());
                }
                RunnerEvent::SettingsWidgets(widgets) => {
                    self.settings
                        .set_widgets(widgets.0.iter().map(Setting::from).collect());
                    // Shown in the Settings tab, not worth a message.
                    continue;
                }
                _ => {}
            }
            self.note(event.describe(), event.is_error());
        }
    }

    /// Adds a message to the recent activity.
    fn note(&mut self, message: String, is_error: bool) {
        if self.recent.len() == RECENT_MESSAGES {
            self.recent.pop_front();
        }
        self.recent.push_back((message, is_error));
    }

    /// The configuration folder, or `None` after reporting that there is
    /// none.
    fn config(&mut self) -> Option<Config> {
        if self.config.is_none() {
            self.note(
                "Couldn't find the configuration folder, so games and settings aren't saved"
                    .to_owned(),
                true,
            );
        }
        self.config.clone()
    }

    /// Reads `splitters.toml`. If it can't be read, the error is reported
    /// and no auto splitter is known.
    fn splitters(&mut self, config: &Config) -> Splitters {
        Splitters::load(config).unwrap_or_else(|error| {
            self.note(
                format!("Couldn't read the games of auto splitters: {error}"),
                true,
            );
            Splitters::default()
        })
    }

    /// Reads a game's file. If it can't be read, the error is reported and
    /// the game has no saved settings.
    fn game(&mut self, config: &Config, slug: &str) -> Game {
        Game::load(config, slug).unwrap_or_else(|error| {
            self.note(format!("Couldn't read the game's settings: {error}"), true);
            Game {
                slug: slug.to_owned(),
                name: slug.to_owned(),
                settings: toml::Table::new(),
            }
        })
    }

    /// The games already set up, for the Game dialog.
    fn games(&mut self, config: &Config, splitters: &Splitters) -> Vec<GameSummary> {
        let (games, errors) = Game::list(config, splitters);
        for error in errors {
            self.note(format!("Couldn't read a game: {error}"), true);
        }
        games
    }

    /// Loads the auto splitter at `path` with its game's settings (spec
    /// §7.3). A file with no game yet asks which game it is for first.
    fn load(&mut self, path: PathBuf) {
        let Some(config) = self.config() else {
            self.runner.load(path, Default::default());
            return;
        };
        let splitters = self.splitters(&config);
        match splitters.game_of(&path) {
            Some(slug) => {
                let game = self.game(&config, slug);
                self.load_with(path, game);
            }
            None => {
                let games = self.games(&config, &splitters);
                self.dialog = Some((GameDialog::new(&file_name(&path), games), path));
            }
        }
    }

    fn load_with(&mut self, path: PathBuf, game: Game) {
        self.runner
            .load(path.clone(), config::to_runtime(&game.settings));
        self.loading.insert(path, game);
    }

    /// Opens the "Change game" dialog for the loaded auto splitter.
    fn change_game(&mut self) {
        let Some(path) = self.runner.loaded_path() else {
            return;
        };
        let Some(config) = self.config() else {
            return;
        };
        let splitters = self.splitters(&config);
        let games = self.games(&config, &splitters);
        let current = self.game.as_ref().map(|game| {
            games
                .iter()
                .find(|summary| summary.slug == game.slug)
                .cloned()
                .unwrap_or_else(|| GameSummary {
                    slug: game.slug.clone(),
                    name: game.name.clone(),
                    splitters: splitters.count_for(&game.slug),
                })
        });
        let dialog = match &current {
            Some(current) => GameDialog::change(&file_name(&path), games, current),
            None => GameDialog::new(&file_name(&path), games),
        };
        self.dialog = Some((dialog, path));
    }

    /// Shows the Game dialog while it is open, and acts on its answer.
    fn game_dialog(&mut self, ctx: &egui::Context) {
        let Some((dialog, _)) = &mut self.dialog else {
            return;
        };
        let Some(action) = components::game_dialog(ctx, dialog) else {
            return;
        };
        let Some((dialog, path)) = self.dialog.take() else {
            return;
        };
        let GameDialogAction::Use(choice) = action else {
            return;
        };
        let Some(config) = self.config() else {
            return;
        };
        let game = match choice {
            GameChoice::Existing(slug) => self.game(&config, &slug),
            GameChoice::New(name) => match Game::create(&config, &name) {
                Ok(game) => game,
                Err(error) => {
                    self.note(format!("Couldn't set up {name}: {error}"), true);
                    return;
                }
            },
        };
        self.remember_game(&config, &path, &game);
        match dialog.kind() {
            GameDialogKind::New => self.load_with(path, game),
            GameDialogKind::Change => {
                // The running auto splitter carries on with the new game's
                // settings.
                self.runner.set_settings(config::to_runtime(&game.settings));
                self.note(
                    format!("{} is now for {}", file_name(&path), game.name),
                    false,
                );
                self.settings.reset_keeping_widgets(game.settings.clone());
                self.game = Some(game);
            }
        }
    }

    /// Saves in `splitters.toml` that the auto splitter at `path` is for
    /// `game`. A file that couldn't be read is left as it is, not replaced.
    fn remember_game(&mut self, config: &Config, path: &std::path::Path, game: &Game) {
        let saved = Splitters::load(config).and_then(|mut splitters| {
            splitters.associate(path, &game.slug)?;
            splitters.save(config)
        });
        if let Err(error) = saved {
            self.note(
                format!("Couldn't remember the game of {}: {error}", file_name(path)),
                true,
            );
        }
    }

    /// The status column: the header, the error card while there is one, the
    /// auto splitter and the game. It scrolls rather than clip a card.
    fn status_column(&mut self, ui: &mut egui::Ui, width: Width) {
        let loaded = self.runner.loaded_path().map(|path| file_name(&path));
        let loaded = loaded.as_deref();
        let game = self.game.as_ref().map(|game| game.name.clone());
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            components::app_header(ui, width);

            if let Some(error) = self.status.error() {
                match components::error_card(ui, error, width) {
                    Some(ErrorAction::Reload) => self.ask(Pending::Reload),
                    Some(ErrorAction::Dismiss) => self.status.dismiss_error(),
                    None => {}
                }
            }

            let action = match width {
                Width::Wide => components::splitter_card(ui, loaded, game.as_deref()),
                Width::Compact => components::splitter_strip(ui, loaded),
            };
            match action {
                Some(SplitterAction::Open) => self.open(),
                Some(SplitterAction::Reload) => self.ask(Pending::Reload),
                Some(SplitterAction::Change) => self.change_game(),
                None => {}
            }

            let detail = self.status.game_detail(width == Width::Compact);
            components::game_card(ui, &self.status.game(), detail.as_deref(), width);
        });
    }

    /// Asks for a `.wasm` file and loads it, once the unsaved settings are
    /// dealt with.
    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Auto splitter", &["wasm"])
            .pick_file()
        {
            self.ask(Pending::Open(path));
        }
    }

    /// Does `pending` now, or first asks what to do with the unsaved
    /// settings, if there are some.
    fn ask(&mut self, pending: Pending) {
        if self.settings.is_unsaved() {
            self.pending = Some(pending);
        } else {
            self.proceed(pending);
        }
    }

    fn proceed(&mut self, pending: Pending) {
        match pending {
            Pending::Reload => self.reload(),
            Pending::Open(path) => self.load(path),
            Pending::Close => {
                self.closing = true;
                self.ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    /// Asks what to do with unsaved settings when the window is closed.
    fn check_close(&mut self) {
        let requested = self.ctx.input(|input| input.viewport().close_requested());
        if requested && !self.closing && self.settings.is_unsaved() {
            self.ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.pending = Some(Pending::Close);
        }
    }

    /// Shows the Unsaved dialog while it is open, and acts on its answer.
    fn unsaved_dialog(&mut self) {
        let Some(pending) = &self.pending else {
            return;
        };
        if let Some(action) =
            components::unsaved_dialog(&self.ctx, pending.trigger(), self.settings.changes())
        {
            self.answer_unsaved(action);
        }
    }

    /// Acts on the Unsaved dialog's answer: saves or discards the edits,
    /// then does what was waiting, unless it was cancelled or saving failed.
    fn answer_unsaved(&mut self, action: UnsavedAction) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        match action {
            UnsavedAction::Save => {
                if self.save_settings() {
                    self.proceed(pending);
                }
            }
            UnsavedAction::Discard => {
                self.settings.discard();
                self.proceed(pending);
            }
            UnsavedAction::Cancel => {}
        }
    }

    /// Saves the draft in the game's file (spec §7.2) and gives it to the
    /// running auto splitter. Returns whether it was saved.
    fn save_settings(&mut self) -> bool {
        let saved = match (self.config.clone(), &self.game) {
            (Some(config), Some(game)) => {
                match Game::save_settings(
                    &config,
                    &game.slug,
                    self.settings.draft(),
                    self.settings.keys(),
                ) {
                    Ok(game) => {
                        let saved = game.settings.clone();
                        self.game = Some(game);
                        saved
                    }
                    Err(error) => {
                        self.note(format!("Couldn't save the settings: {error}"), true);
                        return false;
                    }
                }
            }
            // With no configuration folder there is no game file: the
            // settings last until the app closes.
            _ => self.settings.draft().clone(),
        };
        self.runner.set_settings(config::to_runtime(&saved));
        self.settings.saved(saved, Instant::now());
        true
    }

    /// Asks for a file for the file selection `key`, and puts it in the
    /// draft.
    fn browse(&mut self, key: &str) {
        let Some(widget) = self
            .settings
            .widgets()
            .iter()
            .find(|widget| &*widget.key == key)
            .cloned()
        else {
            return;
        };
        let WidgetKind::FileSelect { filters } = &widget.kind else {
            return;
        };
        let mut dialog = rfd::FileDialog::new().set_title(&*widget.description);
        for (name, extensions) in settings::file_filters(filters) {
            dialog = dialog.add_filter(name, &extensions);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        // The auto splitter sees files through its own file system.
        match wasi_path::from_native(&path) {
            Some(value) => self
                .settings
                .set(&widget, Some(Value::String(value.into()))),
            None => self.note(
                format!(
                    "Couldn't give {} to the auto splitter: the path isn't supported",
                    path.display()
                ),
                true,
            ),
        }
    }

    /// The tab area: the tab strip and the active tab.
    fn tabs(&mut self, ui: &mut egui::Ui) {
        let tabs = [
            Tab {
                id: TabId::Settings,
                label: "Settings",
                unsaved: self.settings.is_unsaved(),
            },
            Tab {
                id: TabId::Log,
                label: "Log",
                unsaved: false,
            },
        ];
        components::tab_strip(ui, &tabs, &mut self.tab);
        ui.add_space(24.0);
        match self.tab {
            TabId::Settings => {
                let loaded = self.runner.loaded_path().is_some();
                match settings_tab::settings_tab(ui, &mut self.settings, loaded, Instant::now()) {
                    Some(SettingsAction::Save) => {
                        self.save_settings();
                    }
                    Some(SettingsAction::Open) => self.open(),
                    Some(SettingsAction::Browse(key)) => self.browse(&key),
                    None => {}
                }
            }
            TabId::Log => self.recent_activity(ui),
        }
    }

    /// Loads the same file again, with its game's saved settings.
    fn reload(&mut self) {
        if let Some(path) = self.runner.loaded_path() {
            self.load(path);
        }
    }

    /// The Runner's recent messages, until the Log tab (#12) replaces them.
    fn recent_activity(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for (message, is_error) in &self.recent {
                    let color = if *is_error {
                        theme::STATUS_ERROR
                    } else {
                        theme::TEXT
                    };
                    ui.add(
                        Label::new(RichText::new(message).font(theme::mono(12.0)).color(color))
                            .wrap(),
                    );
                }
            });
    }
}

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();
        self.check_close();
        self.game_dialog(&ui.ctx().clone());
        self.unsaved_dialog();
        let column = Frame::NONE
            .fill(theme::PANEL)
            .inner_margin(Margin::same(16));
        if ui.max_rect().width() < COMPACT_BELOW {
            egui::CentralPanel::default()
                .frame(column)
                .show(ui, |ui| self.status_column(ui, Width::Compact));
            return;
        }
        egui::Panel::left("status")
            .exact_size(COLUMN_WIDTH)
            .resizable(false)
            .frame(column)
            .show(ui, |ui| self.status_column(ui, Width::Wide));
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::WINDOW).inner_margin(Margin {
                top: 8,
                ..Margin::same(24)
            }))
            .show(ui, |ui| self.tabs(ui));
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{RawInput, ViewportEvent, ViewportId};

    use super::*;

    /// The app with its files in a temporary folder, the auto splitter of
    /// `game` loaded and its setting "gym" changed but not saved.
    fn app_with_unsaved_edit() -> (tempfile::TempDir, BridgeApp) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        let mut app = BridgeApp::with_config(&egui::Context::default(), Some(config.clone()));
        let game = Game::create(&config, "GTA").unwrap();
        app.settings.reset(game.settings.clone());
        app.game = Some(game);
        let gym = Setting {
            key: "gym".into(),
            description: "Gym Moves".into(),
            tooltip: None,
            kind: WidgetKind::Bool {
                default_value: false,
            },
        };
        app.settings.set_widgets(vec![gym.clone()]);
        app.settings.set(&gym, Some(Value::Boolean(true)));
        (dir, app)
    }

    fn saved_gym(app: &BridgeApp) -> Option<bool> {
        let config = app.config.as_ref().unwrap();
        Game::load(config, "gta")
            .unwrap()
            .settings
            .get("gym")?
            .as_bool()
    }

    #[test]
    fn reload_and_open_ask_first_with_unsaved_settings() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.ask(Pending::Reload);
        assert_eq!(app.pending, Some(Pending::Reload));
        let path = PathBuf::from("/other.wasm");
        app.ask(Pending::Open(path.clone()));
        assert_eq!(app.pending, Some(Pending::Open(path)));
    }

    #[test]
    fn nothing_asks_without_unsaved_settings() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.settings.discard();
        app.ask(Pending::Reload);
        assert_eq!(app.pending, None);
    }

    #[test]
    fn closing_the_window_asks_first_with_unsaved_settings() {
        let (_dir, mut app) = app_with_unsaved_edit();
        let mut input = RawInput::default();
        input
            .viewports
            .entry(ViewportId::ROOT)
            .or_default()
            .events
            .push(ViewportEvent::Close);
        let ctx = app.ctx.clone();
        let mut output = ctx.run_ui(input, |_| app.check_close());
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        assert_eq!(app.pending, Some(Pending::Close));
        let commands = &output.viewport_output[&ViewportId::ROOT].commands;
        assert!(
            commands.contains(&ViewportCommand::CancelClose),
            "{commands:?}"
        );
    }

    #[test]
    fn save_writes_the_settings_then_does_what_was_waiting() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.pending = Some(Pending::Close);
        app.answer_unsaved(UnsavedAction::Save);
        assert_eq!(saved_gym(&app), Some(true));
        assert!(!app.settings.is_unsaved());
        assert!(app.closing);
    }

    #[test]
    fn discard_drops_the_edits_then_does_what_was_waiting() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.pending = Some(Pending::Close);
        app.answer_unsaved(UnsavedAction::Discard);
        assert_eq!(saved_gym(&app), None);
        assert!(!app.settings.is_unsaved());
        assert!(app.closing);
    }

    #[test]
    fn cancel_keeps_the_edits_and_does_nothing() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.pending = Some(Pending::Close);
        app.answer_unsaved(UnsavedAction::Cancel);
        assert_eq!(app.pending, None);
        assert!(app.settings.is_unsaved());
        assert!(!app.closing);
    }

    #[test]
    fn title_names_livesplit_one() {
        assert!(window_title().contains("LiveSplit One"));
    }

    #[test]
    fn title_is_display_name_and_version() {
        assert_eq!(
            window_title(),
            format!("LiveSplit One ASR Bridge {VERSION}")
        );
    }
}
