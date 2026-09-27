//! The application window: the status column and, when wide, the tab
//! area.

mod components;
mod connection_tab;
mod file_pick;
mod folder;
mod last_action;
mod preferences_tab;
mod server_status;
mod settings;
mod settings_tab;
mod status;
mod theme;

use std::{
    collections::HashMap,
    fs,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::mpsc::Receiver,
    thread,
    time::{Duration, Instant},
};

use eframe::egui::{self, Frame, Margin, ScrollArea, ViewportCommand};
use livesplit_auto_splitting::{settings::WidgetKind, wasi_path};
use toml::Value;

use crate::{
    config::{self, AppSettings, Config, Game, GameSummary, Splitters, WindowGeometry},
    logging::{self, Category, Filters, Logger},
    runner::{Runner, RunnerEvent, TimerAction, file_name},
    server::{self, NetworkAddress, Server, ServerEvent},
    version::VERSION,
};
use components::{
    ErrorAction, GameChoice, GameDialog, GameDialogAction, GameDialogKind, LOG_LINE_HEIGHT,
    LogAction, SplitterAction, UnsavedAction, UnsavedTrigger, Width,
};
use connection_tab::ConnectionTab;
use file_pick::{FilePick, PickFor, PickPoll};
use last_action::LastActions;
use preferences_tab::PreferencesAction;
use server_status::ServerStatus;
use settings::{Setting, SettingsEditor};
use settings_tab::SettingsAction;
use status::Status;

pub use theme::install as install_theme;

/// The application name, used for the window and the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The name shown to people, in the window title.
pub const DISPLAY_NAME: &str = "LiveSplit One ASR Bridge";

/// How often the machine's addresses are read again, so a VPN coming up
/// appears.
const ADDRESSES_REFRESH: Duration = Duration::from_secs(5);

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

/// The window's size when it is remembered but nothing is saved yet.
const DEFAULT_WINDOW_SIZE: [f32; 2] = [800.0, 600.0];

/// The window as it opens (spec §6.7): with the preference on, at its saved
/// size and position, or the default size when none is saved. With it off,
/// the app sets neither and leaves them to the window manager.
pub fn window_viewport(
    viewport: egui::ViewportBuilder,
    settings: &AppSettings,
) -> egui::ViewportBuilder {
    if !settings.remember_window() {
        return viewport;
    }
    let geometry = settings.window_geometry();
    let viewport = viewport.with_inner_size(geometry.size.unwrap_or(DEFAULT_WINDOW_SIZE));
    match geometry.position {
        Some(position) => viewport.with_position(position),
        None => viewport,
    }
}

/// The tabs of the tab area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    Connection,
    Log,
    Preferences,
}

/// What waits for an answer in the Unsaved dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pending {
    Reload,
    /// Ask for a `.wasm` file and load it.
    Open,
    /// Change the loaded auto splitter's game: the game's settings replace
    /// the draft.
    ChangeGame,
    Close,
}

impl Pending {
    fn trigger(&self) -> UnsavedTrigger {
        match self {
            Self::Reload => UnsavedTrigger::Reload,
            Self::Open => UnsavedTrigger::Open,
            Self::ChangeGame => UnsavedTrigger::ChangeGame,
            Self::Close => UnsavedTrigger::Close,
        }
    }
}

/// The application. Later issues add the other tabs.
pub struct BridgeApp {
    ctx: egui::Context,
    runner: Runner,
    events: Receiver<RunnerEvent>,
    server: Server,
    server_events: Receiver<ServerEvent>,
    status: Status,
    server_status: ServerStatus,
    last_actions: LastActions,
    /// Whether the latest timer action set the game time, so another game
    /// time right after it isn't logged.
    setting_game_time: bool,
    /// The machine's addresses, and when they were read.
    addresses: Vec<NetworkAddress>,
    addresses_read: Instant,
    /// The tab shown in the tab area.
    tab: Tab,
    connection_tab: ConnectionTab,
    /// Everything the app records, in the Log tab and on disk.
    logger: Logger,
    /// The Log tab's filters, saved in `app.toml`.
    log_filters: Filters,
    /// The log entry reached through Show in log (#14), highlighted.
    highlighted: Option<u64>,
    /// The configuration folder, if the OS has one for the user.
    config: Option<Config>,
    /// The app's own settings, read from `app.toml` at start-up. The
    /// server listens on its port, and Restart server saves it.
    app_settings: AppSettings,
    /// Whether the window's size and position are remembered, as shown in
    /// the Preferences tab.
    remember_window: bool,
    /// The window's size and position as last seen, saved when the app
    /// closes.
    window: WindowGeometry,
    /// The game of each auto splitter being loaded, until it is loaded.
    loading: HashMap<PathBuf, Game>,
    /// The loaded auto splitter's game.
    game: Option<Game>,
    /// The open Game dialog, and the file it is for.
    dialog: Option<(GameDialog, PathBuf)>,
    /// The loaded auto splitter's settings, and the edits not saved yet.
    settings: SettingsEditor,
    /// What the open Unsaved dialog is asking about.
    pending: Option<Pending>,
    /// Closing was asked for and the unsaved settings dealt with.
    closing: bool,
    /// The file picker asked for this frame, which opens at the end of it.
    requested: Option<PickFor>,
    /// The file picker for Open…, Save log… or Browse, while it is open.
    file_pick: FilePick,
}

impl BridgeApp {
    /// The application, with its files in `config` and `app.toml` as read
    /// from there, if there is a configuration folder.
    pub fn new(
        ctx: &egui::Context,
        config: Option<Config>,
        loaded: Option<Result<AppSettings, String>>,
    ) -> Self {
        let wake = {
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        };
        let app_settings = match &loaded {
            Some(Ok(settings)) => settings.clone(),
            _ => AppSettings::default(),
        };
        let port = app_settings.port();
        let (server, server_events) = Server::new(port, wake.clone());
        // The auto splitter controls the timers connected to the Server.
        let (runner, events) = Runner::new(server.link(), wake);
        let mut app = Self {
            ctx: ctx.clone(),
            runner,
            events,
            server,
            server_events,
            status: Status::default(),
            server_status: ServerStatus::default(),
            last_actions: LastActions::default(),
            setting_game_time: false,
            addresses: server::network_addresses(),
            addresses_read: Instant::now(),
            tab: Tab::Connection,
            connection_tab: ConnectionTab::new(port),
            logger: Logger::new(logging::standard_dir()),
            log_filters: app_settings.log_filters(),
            highlighted: None,
            config,
            remember_window: app_settings.remember_window(),
            window: app_settings.window_geometry(),
            app_settings,
            loading: HashMap::new(),
            game: None,
            dialog: None,
            settings: SettingsEditor::default(),
            pending: None,
            closing: false,
            requested: None,
            file_pick: FilePick::default(),
        };
        // `app.toml` is read before the server starts, since the server
        // listens on its port; what went wrong is reported once the app can.
        match loaded {
            // Reports that there is no configuration folder.
            None => {
                let _ = app.config();
            }
            Some(Err(error)) => app.log(
                Category::Error,
                format!("Couldn't read the app's settings: {error}"),
            ),
            Some(Ok(_)) => {}
        }
        app
    }

    /// Saves the port the server was restarted on in `app.toml`, so the
    /// next start listens on it too.
    fn save_port(&mut self, port: u16) {
        self.app_settings.set_port(port);
        let Some(config) = self.config() else {
            return;
        };
        if let Err(error) = self.app_settings.save(&config) {
            self.log(Category::Error, format!("Couldn't save the port: {error}"));
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.status.apply(&event);
            self.last_actions
                .apply(&event, chrono::Local::now().time(), Instant::now());
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
                RunnerEvent::TimerAction { action, .. } => {
                    if repeats_game_time(&mut self.setting_game_time, action) {
                        continue;
                    }
                }
                RunnerEvent::SettingsWidgets(widgets) => {
                    self.settings
                        .set_widgets(widgets.0.iter().map(Setting::from).collect());
                    // Shown in the Settings tab, not worth a message.
                    continue;
                }
                _ => {}
            }
            self.log(category(&event), event.describe());
        }
        while let Ok(event) = self.server_events.try_recv() {
            self.status.apply_server(&event);
            self.server_status.apply(&event);
            self.log(server_category(&event), event.describe());
        }
    }

    /// Records a line in the log.
    fn log(&mut self, category: Category, message: String) {
        self.logger.record(category, message);
    }

    /// The configuration folder, or `None` after reporting that there is
    /// none.
    fn config(&mut self) -> Option<Config> {
        if self.config.is_none() {
            self.log(
                Category::Error,
                "Couldn't find the configuration folder, so games and settings aren't saved"
                    .to_owned(),
            );
        }
        self.config.clone()
    }

    /// Reads `splitters.toml`. If it can't be read, the error is reported
    /// and no auto splitter is known.
    fn splitters(&mut self, config: &Config) -> Splitters {
        Splitters::load(config).unwrap_or_else(|error| {
            self.log(
                Category::Error,
                format!("Couldn't read the games of auto splitters: {error}"),
            );
            Splitters::default()
        })
    }

    /// Reads a game's file. If it can't be read, the error is reported and
    /// the game has no saved settings.
    fn game(&mut self, config: &Config, slug: &str) -> Game {
        Game::load(config, slug).unwrap_or_else(|error| {
            self.log(
                Category::Error,
                format!("Couldn't read the game's settings: {error}"),
            );
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
            self.log(Category::Error, format!("Couldn't read a game: {error}"));
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
                    self.log(Category::Error, format!("Couldn't set up {name}: {error}"));
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
                self.log(
                    Category::App,
                    format!("{} is now for {}", file_name(&path), game.name),
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
            self.log(
                Category::Error,
                format!("Couldn't remember the game of {}: {error}", file_name(path)),
            );
        }
    }

    /// Reads the machine's addresses again every few seconds.
    fn refresh_addresses(&mut self, ctx: &egui::Context) {
        if self.addresses_read.elapsed() >= ADDRESSES_REFRESH {
            self.addresses = server::network_addresses();
            self.addresses_read = Instant::now();
        }
        ctx.request_repaint_after(ADDRESSES_REFRESH.saturating_sub(self.addresses_read.elapsed()));
    }

    /// The status column: the header, the error card while there is one, the
    /// auto splitter, the game, the timer and the last action. It scrolls
    /// rather than clip a card.
    fn status_column(&mut self, ui: &mut egui::Ui, width: Width) {
        let loaded = self.runner.loaded_path().map(|path| file_name(&path));
        let loaded = loaded.as_deref();
        let game = self.game.as_ref().map(|game| game.name.clone());
        let picking = self.file_pick.is_open();
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            components::app_header(ui, width);

            if let Some(error) = self.status.error() {
                match components::error_card(ui, error, width) {
                    Some(ErrorAction::Reload) => self.ask(Pending::Reload),
                    Some(ErrorAction::OpenConnection) => self.tab = Tab::Connection,
                    Some(ErrorAction::Dismiss) => self.status.dismiss_error(),
                    None => {}
                }
            }

            let action = match width {
                Width::Wide => components::splitter_card(ui, loaded, game.as_deref(), picking),
                Width::Compact => components::splitter_strip(ui, loaded, picking),
            };
            match action {
                Some(SplitterAction::Open) => self.open(),
                Some(SplitterAction::Reload) => self.ask(Pending::Reload),
                Some(SplitterAction::Change) => self.ask(Pending::ChangeGame),
                None => {}
            }

            let detail = self.status.game_detail(width == Width::Compact);
            components::game_card(ui, &self.status.game(), detail.as_deref(), width);

            let urls = self.server_status.urls(&self.addresses);
            components::timer_card(ui, self.server_status.timer_state(), &urls, width);

            components::last_action_card(ui, &self.last_actions, width, Instant::now());
        });
    }

    /// The tab area: the tab strip, then the active tab.
    fn tab_area(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.0;
        components::tab_strip(
            ui,
            &[
                (Tab::Settings, "Settings", self.settings.is_unsaved()),
                (Tab::Connection, "Connection", false),
                (Tab::Log, "Log", false),
                (Tab::Preferences, "Preferences", false),
            ],
            &mut self.tab,
        );
        Frame::NONE
            .inner_margin(Margin::same(24))
            .show(ui, |ui| match self.tab {
                Tab::Settings => self.settings_tab(ui),
                Tab::Connection => {
                    let urls = self.server_status.urls(&self.addresses);
                    if let Some(port) = self.connection_tab.show(ui, &self.server_status, &urls) {
                        self.server.restart(port);
                        self.save_port(port);
                    }
                }
                Tab::Log => self.log_tab(ui),
                Tab::Preferences => self.preferences_tab(ui),
            });
    }

    /// The Settings tab, and what it asks for.
    fn settings_tab(&mut self, ui: &mut egui::Ui) {
        let loaded = self.runner.loaded_path().is_some();
        match settings_tab::settings_tab(ui, &mut self.settings, loaded, Instant::now()) {
            Some(SettingsAction::Save) => {
                self.save_settings();
            }
            Some(SettingsAction::Open) => self.open(),
            Some(SettingsAction::Browse(key)) => self.request_pick(PickFor::Setting(key)),
            None => {}
        }
    }

    /// The Preferences tab, and what it asks for.
    fn preferences_tab(&mut self, ui: &mut egui::Ui) {
        let config_dir = self.config.as_ref().map(|config| config.dir().to_owned());
        let log_dir = self.logger.dir().map(ToOwned::to_owned);
        match preferences_tab::preferences_tab(
            ui,
            &mut self.remember_window,
            config_dir.as_deref(),
            log_dir.as_deref(),
        ) {
            Some(PreferencesAction::RememberWindowChanged) => self.save_remember_window(),
            Some(PreferencesAction::OpenConfigFolder) => {
                if let Some(dir) = config_dir
                    && let Err(error) = folder::open_folder(&dir)
                {
                    self.log(Category::Error, error);
                }
            }
            Some(PreferencesAction::OpenLogFolder) => self.open_log_folder(),
            None => {}
        }
    }

    /// Saves in `app.toml` whether the window's size and position are
    /// remembered. It applies the next time the app starts.
    fn save_remember_window(&mut self) {
        self.app_settings.set_remember_window(self.remember_window);
        let Some(config) = self.config() else {
            return;
        };
        if let Err(error) = self.app_settings.save(&config) {
            self.log(
                Category::Error,
                format!("Couldn't save the window preference: {error}"),
            );
        }
    }

    /// Notes the window's size and position, as far as the OS tells them.
    /// A minimized window's are left out: Windows, for example, puts it far
    /// off screen.
    fn track_window(&mut self, ctx: &egui::Context) {
        let (inner, outer, minimized) = ctx.input(|input| {
            let viewport = input.viewport();
            (viewport.inner_rect, viewport.outer_rect, viewport.minimized)
        });
        if minimized == Some(true) {
            return;
        }
        if let Some(inner) = inner {
            self.window.size = Some(inner.size().into());
        }
        if let Some(outer) = outer {
            self.window.position = Some(outer.min.into());
        }
    }

    /// Saves the window's size and position in `app.toml` when they are
    /// remembered, so the next start opens the window as it was.
    fn save_window(&mut self) {
        if !self.remember_window || self.window == self.app_settings.window_geometry() {
            return;
        }
        self.app_settings.set_window_geometry(self.window);
        let Some(config) = self.config.clone() else {
            return;
        };
        if let Err(error) = self.app_settings.save(&config) {
            self.log(
                Category::Error,
                format!("Couldn't remember the window's size and position: {error}"),
            );
        }
    }

    /// Asks for a `.wasm` file and loads it, once the unsaved settings are
    /// dealt with, so the file dialog comes after the Unsaved dialog.
    fn open(&mut self) {
        self.ask(Pending::Open);
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
            Pending::Open => self.request_pick(PickFor::Open),
            Pending::ChangeGame => self.change_game(),
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
        let game = self.game.as_ref().map(|game| game.name.as_str());
        if let Some(action) =
            components::unsaved_dialog(&self.ctx, pending.trigger(), self.settings.changes(), game)
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
                        self.log(
                            Category::Error,
                            format!("Couldn't save the settings: {error}"),
                        );
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
        self.log(Category::App, "Saved the settings".to_owned());
        true
    }

    /// Asks for the file picker for `for_`. The status column, the tabs and
    /// the Unsaved dialog have no `Frame` to parent the picker to, so it
    /// opens at the end of the frame. The first request of a frame wins.
    fn request_pick(&mut self, for_: PickFor) {
        self.requested.get_or_insert(for_);
    }

    /// The file picker for `for_`, not shown yet, or `None` if what it is
    /// for has gone.
    fn pick_dialog(&self, for_: &PickFor) -> Option<rfd::AsyncFileDialog> {
        let dialog = rfd::AsyncFileDialog::new();
        match for_ {
            PickFor::Open => Some(dialog.add_filter("Auto splitter", &["wasm"])),
            PickFor::SaveLog => Some(
                dialog
                    .add_filter("Log", &["log", "txt"])
                    .set_file_name(format!("{APP_NAME}.log")),
            ),
            PickFor::Setting(key) => {
                let widget = self.file_setting(key)?;
                let WidgetKind::FileSelect { filters } = &widget.kind else {
                    return None;
                };
                let mut dialog = dialog.set_title(&*widget.description);
                for (name, extensions) in settings::file_filters(filters) {
                    dialog = dialog.add_filter(name, &extensions);
                }
                Some(dialog)
            }
        }
    }

    /// Opens the file picker for `for_`, unless one is already open. The
    /// picker waits on its own thread, so the window keeps answering the
    /// compositor (#50, #59); [`Self::poll_file_pick`] acts on the file
    /// picked.
    fn start_pick(&mut self, frame: &eframe::Frame, ctx: &egui::Context, for_: PickFor) {
        if self.file_pick.is_open() {
            return;
        }
        let Some(dialog) = self.pick_dialog(&for_) else {
            return;
        };
        let saving = for_ == PickFor::SaveLog;
        let Some(sender) = self.file_pick.begin(for_) else {
            return;
        };
        // Made here, on the UI thread: on macOS the picker is a sheet on the
        // window, shown as soon as it is made.
        let dialog = dialog.set_parent(frame);
        let pick: Pin<Box<dyn Future<Output = Option<rfd::FileHandle>> + Send>> = if saving {
            Box::pin(dialog.save_file())
        } else {
            Box::pin(dialog.pick_file())
        };
        let ctx = ctx.clone();
        let spawned = thread::Builder::new()
            .name("file picker".to_owned())
            .spawn(move || {
                let path = pollster::block_on(pick).map(|file| file.path().to_owned());
                let _ = sender.send(path);
                ctx.request_repaint();
            });
        if let Err(error) = spawned {
            self.file_pick.cancel();
            self.log(
                Category::Error,
                format!("Couldn't open the file picker: {error}"),
            );
        }
    }

    /// Acts on the file picked, once the file picker has closed.
    fn poll_file_pick(&mut self) {
        let PickPoll::Picked(for_, path) = self.file_pick.poll() else {
            return;
        };
        match for_ {
            PickFor::Open => self.load(path),
            PickFor::SaveLog => self.write_log(&path),
            PickFor::Setting(key) => self.set_file_setting(&key, &path),
        }
    }

    /// The file selection setting `key`, if the auto splitter still has it.
    fn file_setting(&self, key: &str) -> Option<Setting> {
        self.settings
            .widgets()
            .iter()
            .find(|widget| &*widget.key == key)
            .filter(|widget| matches!(widget.kind, WidgetKind::FileSelect { .. }))
            .cloned()
    }

    /// Puts the file at `path` in the draft, for the file selection `key`.
    /// If the setting has gone while the picker was open, the file is
    /// dropped.
    fn set_file_setting(&mut self, key: &str, path: &Path) {
        let Some(widget) = self.file_setting(key) else {
            return;
        };
        // The auto splitter sees files through its own file system.
        match wasi_path::from_native(path) {
            Some(value) => self
                .settings
                .set(&widget, Some(Value::String(value.into()))),
            None => self.log(
                Category::Error,
                format!(
                    "Couldn't give {} to the auto splitter: the path isn't supported",
                    path.display()
                ),
            ),
        }
    }

    /// Loads the same file again, with its game's saved settings.
    fn reload(&mut self) {
        if let Some(path) = self.runner.loaded_path() {
            self.load(path);
        }
    }

    /// The Log tab (spec §6.6): the toolbar, then the lines the filters
    /// show, following the newest.
    fn log_tab(&mut self, ui: &mut egui::Ui) {
        match components::log_toolbar(ui, &mut self.log_filters) {
            Some(LogAction::FiltersChanged) => self.save_log_filters(),
            Some(LogAction::Copy) => ui
                .ctx()
                .copy_text(self.logger.view().export(self.log_filters)),
            Some(LogAction::Save) => self.request_pick(PickFor::SaveLog),
            Some(LogAction::Clear) => self.logger.clear(),
            Some(LogAction::OpenFolder) => self.open_log_folder(),
            None => {}
        }
        ui.add_space(12.0);
        let shown: Vec<_> = self.logger.view().shown(self.log_filters).collect();
        ScrollArea::both()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .show_rows(ui, LOG_LINE_HEIGHT, shown.len(), |ui, rows| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for entry in &shown[rows] {
                    components::log_line(ui, entry, self.highlighted == Some(entry.id));
                }
            });
    }

    /// Remembers the Log tab's filters in `app.toml`.
    fn save_log_filters(&mut self) {
        self.app_settings.set_log_filters(self.log_filters);
        let Some(config) = self.config() else {
            return;
        };
        if let Err(error) = self.app_settings.save(&config) {
            self.log(
                Category::Error,
                format!("Couldn't remember the log filters: {error}"),
            );
        }
    }

    /// Saves the lines the filters show at `path`, as they are now.
    fn write_log(&mut self, path: &Path) {
        if let Err(error) = fs::write(path, self.logger.view().export(self.log_filters)) {
            self.log(
                Category::Error,
                format!("Couldn't save the log to {}: {error}", path.display()),
            );
        }
    }

    /// Opens the log folder in the file manager.
    fn open_log_folder(&mut self) {
        let result = match self.logger.dir() {
            Some(dir) => folder::open_folder(dir),
            None => Err("Couldn't find the log folder".to_owned()),
        };
        if let Err(error) = result {
            self.log(Category::Error, error);
        }
    }
}

/// The log category of a Server event (spec §8.1).
fn server_category(event: &ServerEvent) -> Category {
    if event.is_error() {
        return Category::Error;
    }
    match event {
        // Server restarts are the app's.
        ServerEvent::Listening { .. } => Category::App,
        _ => Category::Connection,
    }
}

/// Whether `action` sets the game time right after another game time, so it
/// isn't logged: auto splitters often set it on every tick, which would fill
/// the log (spec §8.1). `setting_game_time` follows the latest action.
fn repeats_game_time(setting_game_time: &mut bool, action: &TimerAction) -> bool {
    let game_time = matches!(action, TimerAction::SetGameTime(_));
    let repeat = game_time && *setting_game_time;
    *setting_game_time = game_time;
    repeat
}

/// The log category of a Runner event (spec §8.1).
fn category(event: &RunnerEvent) -> Category {
    if event.is_error() {
        return Category::Error;
    }
    match event {
        RunnerEvent::AutoSplitterLog(_) => Category::AutoSplitter,
        // Commands sent and dropped (spec §8.1).
        RunnerEvent::TimerAction { .. } => Category::Connection,
        _ => Category::App,
    }
}

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.track_window(ui.ctx());
        self.handle_events();
        self.check_close();
        self.poll_file_pick();
        self.game_dialog(&ui.ctx().clone());
        self.unsaved_dialog();
        self.refresh_addresses(ui.ctx());
        let column = Frame::NONE
            .fill(theme::PANEL)
            .inner_margin(Margin::same(16));
        if ui.max_rect().width() < COMPACT_BELOW {
            egui::CentralPanel::default()
                .frame(column)
                .show(ui, |ui| self.status_column(ui, Width::Compact));
        } else {
            egui::Panel::left("status")
                .exact_size(COLUMN_WIDTH)
                .resizable(false)
                .frame(column)
                .show(ui, |ui| self.status_column(ui, Width::Wide));
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(theme::WINDOW))
                .show(ui, |ui| self.tab_area(ui));
        }
        // The file picker is asked for mid-frame, where there is no `Frame`
        // to parent it to.
        if let Some(for_) = self.requested.take() {
            self.start_pick(frame, ui.ctx(), for_);
        }
    }

    fn on_exit(&mut self) {
        self.save_window();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eframe::egui::{RawInput, ViewportEvent, ViewportId};
    use livesplit_auto_splitting::time;

    use super::*;

    /// The app with its files in `config`.
    fn test_app(config: Config) -> BridgeApp {
        let loaded = Some(AppSettings::load(&config));
        BridgeApp::new(&egui::Context::default(), Some(config), loaded)
    }

    /// The app with its files in a temporary folder, the auto splitter of
    /// `game` loaded and its setting "gym" changed but not saved.
    fn app_with_unsaved_edit() -> (tempfile::TempDir, BridgeApp) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        let mut app = test_app(config.clone());
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
    fn reload_open_and_change_game_ask_first_with_unsaved_settings() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.ask(Pending::Reload);
        assert_eq!(app.pending, Some(Pending::Reload));
        app.ask(Pending::Open);
        assert_eq!(app.pending, Some(Pending::Open));
        app.ask(Pending::ChangeGame);
        assert_eq!(app.pending, Some(Pending::ChangeGame));
    }

    #[test]
    fn open_asks_before_the_file_dialog() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.open();
        assert_eq!(app.pending, Some(Pending::Open));
        assert_eq!(app.requested, None);
    }

    #[test]
    fn answering_open_shows_the_file_dialog() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.open();
        app.answer_unsaved(UnsavedAction::Discard);
        assert_eq!(app.requested, Some(PickFor::Open));
    }

    #[test]
    fn the_file_picked_is_loaded() {
        let (_dir, mut app) = app_with_unsaved_edit();
        let sender = app.file_pick.begin(PickFor::Open).unwrap();
        sender.send(Some(PathBuf::from("/other.wasm"))).unwrap();
        app.poll_file_pick();
        // The file has no game yet, so loading it asks which game it is for.
        let path = app.dialog.as_ref().map(|(_, path)| path.clone());
        assert_eq!(path, Some(PathBuf::from("/other.wasm")));
    }

    #[test]
    fn cancelling_open_shows_no_file_dialog() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.open();
        app.answer_unsaved(UnsavedAction::Cancel);
        assert_eq!(app.pending, None);
        assert_eq!(app.requested, None);
    }

    #[test]
    fn the_log_is_saved_where_picked() {
        let (dir, mut app) = app_with_unsaved_edit();
        // By default the log shows only errors.
        app.log(Category::Error, "Hello from the test".to_owned());
        let path = dir.path().join("saved.log");
        let sender = app.file_pick.begin(PickFor::SaveLog).unwrap();
        sender.send(Some(path.clone())).unwrap();
        app.poll_file_pick();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("Hello from the test"), "{saved}");
    }

    /// A file selection setting, "route".
    fn route() -> Setting {
        Setting {
            key: "route".into(),
            description: "Route".into(),
            tooltip: None,
            kind: WidgetKind::FileSelect {
                filters: Arc::new(Vec::new()),
            },
        }
    }

    #[test]
    fn the_file_picked_for_a_setting_goes_in_the_draft() {
        let (dir, mut app) = app_with_unsaved_edit();
        let mut widgets = app.settings.widgets().to_vec();
        widgets.push(route());
        app.settings.set_widgets(widgets);
        let path = dir.path().join("route.txt");
        let sender = app
            .file_pick
            .begin(PickFor::Setting("route".to_owned()))
            .unwrap();
        sender.send(Some(path.clone())).unwrap();
        app.poll_file_pick();
        let expected = wasi_path::from_native(&path).unwrap();
        assert_eq!(
            app.settings.draft().get("route").and_then(Value::as_str),
            Some(&*expected)
        );
    }

    #[test]
    fn the_file_picked_for_a_setting_that_has_gone_is_dropped() {
        let (dir, mut app) = app_with_unsaved_edit();
        let sender = app
            .file_pick
            .begin(PickFor::Setting("route".to_owned()))
            .unwrap();
        sender.send(Some(dir.path().join("route.txt"))).unwrap();
        app.poll_file_pick();
        assert_eq!(app.settings.draft().get("route"), None);
    }

    #[test]
    fn the_first_file_picker_asked_for_in_a_frame_wins() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.request_pick(PickFor::SaveLog);
        assert_eq!(app.requested, Some(PickFor::SaveLog));
        // Only one picker opens at a time.
        app.request_pick(PickFor::Setting("route".to_owned()));
        assert_eq!(app.requested, Some(PickFor::SaveLog));
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
    fn runner_events_are_logged_in_their_category() {
        let path = PathBuf::from("a.wasm");
        let cases = [
            (
                RunnerEvent::LoadFailed {
                    path: path.clone(),
                    error: "bad".to_owned(),
                },
                Category::Error,
            ),
            (
                RunnerEvent::Crashed {
                    error: "trap".to_owned(),
                },
                Category::Error,
            ),
            (
                RunnerEvent::AutoSplitterLog("hi".to_owned()),
                Category::AutoSplitter,
            ),
            (
                RunnerEvent::Loaded {
                    path,
                    tick_rate: std::time::Duration::from_millis(50),
                },
                Category::App,
            ),
            (
                RunnerEvent::TimerAction {
                    action: TimerAction::Split,
                    sent_to: 0,
                },
                Category::Connection,
            ),
            (RunnerEvent::GameDetached, Category::App),
            (
                RunnerEvent::TickRateChanged(std::time::Duration::from_millis(50)),
                Category::App,
            ),
        ];
        for (event, expected) in cases {
            assert_eq!(category(&event), expected, "{event:?}");
        }
    }

    #[test]
    fn server_events_are_logged_in_their_category() {
        let address = "192.168.1.42:53122".parse().unwrap();
        let cases = [
            (
                ServerEvent::BindFailed {
                    port: 16834,
                    error: "in use".to_owned(),
                    in_use: true,
                },
                Category::Error,
            ),
            (ServerEvent::Listening { port: 16834 }, Category::App),
            (
                ServerEvent::TimerConnected { id: 0, address },
                Category::Connection,
            ),
            (
                ServerEvent::TimerDisconnected { id: 0, address },
                Category::Connection,
            ),
            (
                ServerEvent::HandshakeFailed {
                    address,
                    error: "not a WebSocket".to_owned(),
                },
                Category::Connection,
            ),
            (
                ServerEvent::CommandRejected {
                    id: 0,
                    address,
                    command: Some("split".to_owned()),
                    code: "NoRunInProgress".to_owned(),
                    message: None,
                },
                Category::Connection,
            ),
        ];
        for (event, expected) in cases {
            assert_eq!(server_category(&event), expected, "{event:?}");
        }
    }

    #[test]
    fn a_remembered_window_opens_where_it_was() {
        let mut settings = AppSettings::default();
        let viewport = window_viewport(egui::ViewportBuilder::default(), &settings);
        assert_eq!(viewport.inner_size, Some(egui::vec2(800.0, 600.0)));
        assert_eq!(viewport.position, None);

        settings.set_window_geometry(WindowGeometry {
            size: Some([1000.0, 700.0]),
            position: Some([50.0, 60.0]),
        });
        let viewport = window_viewport(egui::ViewportBuilder::default(), &settings);
        assert_eq!(viewport.inner_size, Some(egui::vec2(1000.0, 700.0)));
        assert_eq!(viewport.position, Some(egui::pos2(50.0, 60.0)));
    }

    #[test]
    fn with_the_preference_off_the_window_manager_places_the_window() {
        let mut settings = AppSettings::default();
        settings.set_window_geometry(WindowGeometry {
            size: Some([1000.0, 700.0]),
            position: Some([50.0, 60.0]),
        });
        settings.set_remember_window(false);
        let viewport = window_viewport(egui::ViewportBuilder::default(), &settings);
        assert_eq!(viewport.inner_size, None);
        assert_eq!(viewport.position, None);
    }

    fn saved_settings(app: &BridgeApp) -> AppSettings {
        AppSettings::load(app.config.as_ref().unwrap()).unwrap()
    }

    #[test]
    fn closing_saves_the_window_when_it_is_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = test_app(Config::at(dir.path().join("config")));
        let geometry = WindowGeometry {
            size: Some([900.0, 640.0]),
            position: Some([10.0, 20.0]),
        };
        app.window = geometry;
        app.save_window();
        assert_eq!(saved_settings(&app).window_geometry(), geometry);
    }

    #[test]
    fn closing_leaves_the_window_unsaved_when_the_preference_is_off() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = test_app(Config::at(dir.path().join("config")));
        app.remember_window = false;
        app.save_remember_window();
        app.window = WindowGeometry {
            size: Some([900.0, 640.0]),
            position: Some([10.0, 20.0]),
        };
        app.save_window();
        let saved = saved_settings(&app);
        assert!(!saved.remember_window());
        assert_eq!(saved.window_geometry(), WindowGeometry::default());
    }

    #[test]
    fn only_the_first_of_several_game_times_in_a_row_is_logged() {
        let game_time = |seconds| TimerAction::SetGameTime(time::Duration::seconds(seconds));
        let mut setting_game_time = false;
        let logged: Vec<_> = [
            TimerAction::Start,
            game_time(1),
            game_time(2),
            game_time(3),
            TimerAction::Split,
            game_time(4),
            game_time(5),
        ]
        .into_iter()
        .filter(|action| !repeats_game_time(&mut setting_game_time, action))
        .collect();
        assert_eq!(
            logged,
            [
                TimerAction::Start,
                game_time(1),
                TimerAction::Split,
                game_time(4)
            ]
        );
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
