//! The application window: the status column and, when wide, the tab
//! area.

mod components;
mod connection_tab;
mod folder;
mod server_status;
mod settings;
mod settings_tab;
mod status;
mod theme;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, mpsc::Receiver},
    time::{Duration, Instant},
};

use eframe::egui::{self, Frame, Margin, Rect, ScrollArea, UiBuilder, ViewportCommand, vec2};
use livesplit_auto_splitting::{settings::WidgetKind, wasi_path};
use toml::Value;

use crate::{
    config::{self, AppSettings, Config, Game, GameSummary, Splitters},
    logging::{self, Category, Filters, Logger},
    runner::{NoTimer, Runner, RunnerEvent, file_name},
    server::{self, NetworkAddress, Server, ServerEvent},
    version::VERSION,
};
use components::{
    ErrorAction, GameChoice, GameDialog, GameDialogAction, GameDialogKind, HeaderButton, LogAction,
    SplitterAction, UnsavedAction, UnsavedTrigger, Width,
};
use connection_tab::ConnectionTab;
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
/// The window width Show details asks for: the width the window opens at.
const DETAILS_WIDTH: f32 = 800.0;
/// How long Show details waits for the window to widen before showing the
/// tab view instead, as with a tiling window manager.
const WIDEN_WAIT: Duration = Duration::from_millis(500);

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{DISPLAY_NAME} {VERSION}")
}

/// The tabs of the tab area. Preferences comes with #13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Settings,
    Connection,
    Log,
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

/// The application. Later issues add the Last action card and the other
/// tabs.
pub struct BridgeApp {
    ctx: egui::Context,
    runner: Runner,
    events: Receiver<RunnerEvent>,
    server: Server,
    server_events: Receiver<ServerEvent>,
    status: Status,
    server_status: ServerStatus,
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
    /// The log entry reached through Show in log, highlighted.
    highlighted: Option<u64>,
    /// The Log tab scrolls to the highlighted entry the next time it is
    /// shown.
    scroll_to_highlighted: bool,
    /// The log entry of the error the error card shows, for Show in log.
    error_entry: Option<u64>,
    /// In the compact state, the tab view is shown instead of the status
    /// column, until ← Status.
    tab_view: bool,
    /// When Show details asked the window to widen, until it has, or the
    /// tab view is shown instead.
    widening: Option<Instant>,
    /// The configuration folder, if the OS has one for the user.
    config: Option<Config>,
    /// The app's own settings, read from `app.toml` at start-up. The
    /// server listens on its port, and Restart server saves it.
    app_settings: AppSettings,
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
    /// Asks for the `.wasm` file to open. Tests replace the file dialog.
    pick_splitter: fn() -> Option<PathBuf>,
    /// Closing was asked for and the unsaved settings dealt with.
    closing: bool,
}

impl BridgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        Self::with_config(ctx, Config::standard())
    }

    /// The application, with its files in `config`.
    fn with_config(ctx: &egui::Context, config: Option<Config>) -> Self {
        let wake = {
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        };
        // The Server replaces NoTimer (#10, #11).
        let (runner, events) = Runner::new(Arc::new(NoTimer), wake.clone());
        let loaded = config.as_ref().map(AppSettings::load);
        let app_settings = match &loaded {
            Some(Ok(settings)) => settings.clone(),
            _ => AppSettings::default(),
        };
        let port = app_settings.port();
        let (server, server_events) = Server::new(port, wake);
        let mut app = Self {
            ctx: ctx.clone(),
            runner,
            events,
            server,
            server_events,
            status: Status::default(),
            server_status: ServerStatus::default(),
            addresses: server::network_addresses(),
            addresses_read: Instant::now(),
            tab: Tab::Connection,
            connection_tab: ConnectionTab::new(port),
            logger: Logger::new(logging::standard_dir()),
            log_filters: app_settings.log_filters(),
            highlighted: None,
            scroll_to_highlighted: false,
            error_entry: None,
            tab_view: false,
            widening: None,
            config,
            app_settings,
            loading: HashMap::new(),
            game: None,
            dialog: None,
            settings: SettingsEditor::default(),
            pending: None,
            pick_splitter,
            closing: false,
        };
        // `app.toml` is read before the server starts, since the server
        // listens on its port; what went wrong is reported once the app can.
        match loaded {
            // Reports that there is no configuration folder.
            None => {
                let _ = app.config();
            }
            Some(Err(error)) => {
                app.log(
                    Category::Error,
                    format!("Couldn't read the app's settings: {error}"),
                );
            }
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
            self.runner_event(event);
        }
        while let Ok(event) = self.server_events.try_recv() {
            self.server_event(event);
        }
    }

    /// Acts on what the Server reported, and logs it.
    fn server_event(&mut self, event: ServerEvent) {
        self.status.apply_server(&event);
        self.server_status.apply(&event);
        self.log(server_category(&event), event.describe());
    }

    /// Acts on what the Runner reported, and logs it.
    fn runner_event(&mut self, event: RunnerEvent) {
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
                return;
            }
            _ => {}
        }
        let id = self.log(category(&event), event.describe());
        if event.is_error() {
            // The error the card now shows, reached by Show in log.
            self.error_entry = Some(id);
        }
    }

    /// Records a line in the log, and returns its id.
    fn log(&mut self, category: Category, message: String) -> u64 {
        self.logger.record(category, message)
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
    /// auto splitter and the game. It scrolls rather than clip a card.
    fn status_column(&mut self, ui: &mut egui::Ui, width: Width) {
        let loaded = self.runner.loaded_path().map(|path| file_name(&path));
        let loaded = loaded.as_deref();
        let game = self.game.as_ref().map(|game| game.name.clone());
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            if components::app_header(ui, width, Some(HeaderButton::ShowDetails)) {
                self.show_details(ui.ctx());
            }

            if let Some(error) = self.status.error() {
                match components::error_card(ui, error, width) {
                    Some(ErrorAction::ShowInLog) => self.show_in_log(width),
                    Some(ErrorAction::Reload) => self.ask(Pending::Reload),
                    Some(ErrorAction::OpenConnection) => self.open_tab(Tab::Connection, width),
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
                Some(SplitterAction::Change) => self.ask(Pending::ChangeGame),
                None => {}
            }

            let detail = self.status.game_detail(width == Width::Compact);
            components::game_card(ui, &self.status.game(), detail.as_deref(), width);

            let urls = self.server_status.urls(&self.addresses);
            components::timer_card(ui, self.server_status.timer_state(), &urls, width);
        });
    }

    /// Shows `tab`. In the compact state that shows the tab view (spec
    /// §6.3).
    fn open_tab(&mut self, tab: Tab, width: Width) {
        self.tab = tab;
        if width == Width::Compact {
            self.tab_view = true;
        }
    }

    /// Show in log (spec §9): opens the Log tab with the error's category
    /// ticked, scrolled to the error's entry, and the entry highlighted.
    fn show_in_log(&mut self, width: Width) {
        let Some(category) = self.status.error().and_then(|error| error.log_category()) else {
            return;
        };
        let mut filters = self.log_filters;
        filters.show(category);
        if filters != self.log_filters {
            self.log_filters = filters;
            self.save_log_filters();
        }
        self.highlighted = self.error_entry;
        self.scroll_to_highlighted = true;
        self.open_tab(Tab::Log, width);
    }

    /// Show details (spec §6.3): asks the window to widen, and shows the
    /// tab view if it doesn't.
    fn show_details(&mut self, ctx: &egui::Context) {
        let height = ctx.content_rect().height();
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(DETAILS_WIDTH, height)));
        self.widening = Some(Instant::now());
    }

    /// After Show details, shows the tab view once the window has had time
    /// to widen and hasn't.
    fn check_widening(&mut self, ctx: &egui::Context, now: Instant) {
        let Some(asked) = self.widening else {
            return;
        };
        let waited = now.saturating_duration_since(asked);
        if waited >= WIDEN_WAIT {
            self.widening = None;
            self.tab_view = true;
        } else {
            ctx.request_repaint_after(WIDEN_WAIT - waited);
        }
    }

    /// The compact state's tab view: the header with ← Status, then the tab
    /// area, in the space of the status column.
    fn tab_view(&mut self, ui: &mut egui::Ui) {
        let header = Frame::NONE
            .fill(theme::PANEL)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if components::app_header(ui, Width::Compact, Some(HeaderButton::Back)) {
                    self.tab_view = false;
                }
            });
        let rect = header.response.rect;
        ui.painter().hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            egui::Stroke::new(1.0, theme::BORDER),
        );
        self.tab_area(ui, Width::Compact);
    }

    /// The tab area: the tab strip, then the active tab, with less padding
    /// in the compact state's tab view.
    fn tab_area(&mut self, ui: &mut egui::Ui, width: Width) {
        ui.spacing_mut().item_spacing.y = 0.0;
        components::tab_strip(
            ui,
            &[
                (Tab::Settings, "Settings", self.settings.is_unsaved()),
                (Tab::Connection, "Connection", false),
                (Tab::Log, "Log", false),
            ],
            &mut self.tab,
        );
        let padding = match width {
            Width::Wide => 24,
            Width::Compact => 16,
        };
        Frame::NONE
            .inner_margin(Margin::same(padding))
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
            Some(SettingsAction::Browse(key)) => self.browse(&key),
            None => {}
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
            Pending::Open => {
                if let Some(path) = (self.pick_splitter)() {
                    self.load(path);
                }
            }
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
            None => {
                self.log(
                    Category::Error,
                    format!(
                        "Couldn't give {} to the auto splitter: the path isn't supported",
                        path.display()
                    ),
                );
            }
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
            Some(LogAction::Save) => self.save_log(),
            Some(LogAction::Clear) => self.logger.clear(),
            Some(LogAction::OpenFolder) => self.open_log_folder(),
            None => {}
        }
        ui.add_space(12.0);
        let shown: Vec<_> = self.logger.view().shown(self.log_filters).collect();
        // Lines differ in height (a trace, the highlight), so each line's top
        // is added up, and only the lines in view are drawn.
        let heights: Vec<f32> = shown
            .iter()
            .map(|entry| components::log_line_height(entry, self.highlighted == Some(entry.id)))
            .collect();
        let tops: Vec<f32> = std::iter::once(0.0)
            .chain(heights.iter().scan(0.0, |top, height| {
                *top += height;
                Some(*top)
            }))
            .collect();
        let mut scroll = ScrollArea::both().auto_shrink(false);
        // Show in log scrolls once to the highlighted entry, centred;
        // following the newest line would scroll it away again.
        let jump = std::mem::take(&mut self.scroll_to_highlighted)
            .then_some(self.highlighted)
            .flatten()
            .and_then(|id| shown.iter().position(|entry| entry.id == id));
        match jump {
            Some(row) => {
                let offset = tops[row] + heights[row] / 2.0 - ui.available_height() / 2.0;
                scroll = scroll.vertical_scroll_offset(offset.max(0.0));
            }
            None => scroll = scroll.stick_to_bottom(true),
        }
        // Widened into the tab's padding, so the text of the lines lines up
        // with the toolbar (see LOG_LINE_INSET).
        let list = ui
            .available_rect_before_wrap()
            .expand2(vec2(components::LOG_LINE_INSET, 0.0));
        ui.scope_builder(UiBuilder::new().max_rect(list), |ui| {
            scroll.show_viewport(ui, |ui, viewport| {
                ui.set_height(tops[shown.len()]);
                ui.spacing_mut().item_spacing.y = 0.0;
                let first = tops[1..].partition_point(|&bottom| bottom <= viewport.min.y);
                let last = tops
                    .partition_point(|&top| top < viewport.max.y)
                    .min(shown.len());
                let top = ui.max_rect().top();
                for row in first..last {
                    let entry = shown[row];
                    let rect = Rect::from_x_y_ranges(
                        ui.max_rect().x_range(),
                        top + tops[row]..=top + tops[row + 1],
                    );
                    ui.scope_builder(UiBuilder::new().max_rect(rect).id_salt(entry.id), |ui| {
                        components::log_line(ui, entry, self.highlighted == Some(entry.id));
                    });
                }
            });
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

    /// Asks where to save the lines shown, and saves them there.
    fn save_log(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Log", &["log", "txt"])
            .set_file_name(format!("{APP_NAME}.log"))
            .save_file()
        else {
            return;
        };
        if let Err(error) = fs::write(&path, self.logger.view().export(self.log_filters)) {
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

/// The log category of a Runner event (spec §8.1).
fn category(event: &RunnerEvent) -> Category {
    if event.is_error() {
        return Category::Error;
    }
    match event {
        RunnerEvent::AutoSplitterLog(_) | RunnerEvent::TimerAction(_) => Category::AutoSplitter,
        _ => Category::App,
    }
}

/// Asks for a `.wasm` file with the file dialog.
fn pick_splitter() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Auto splitter", &["wasm"])
        .pick_file()
}

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();
        self.check_close();
        self.game_dialog(&ui.ctx().clone());
        self.unsaved_dialog();
        self.refresh_addresses(ui.ctx());
        let column = Frame::NONE
            .fill(theme::PANEL)
            .inner_margin(Margin::same(16));
        if ui.max_rect().width() < COMPACT_BELOW {
            self.check_widening(ui.ctx(), Instant::now());
            if self.tab_view {
                egui::CentralPanel::default()
                    .frame(Frame::NONE.fill(theme::WINDOW))
                    .show(ui, |ui| self.tab_view(ui));
            } else {
                egui::CentralPanel::default()
                    .frame(column)
                    .show(ui, |ui| self.status_column(ui, Width::Compact));
            }
            return;
        }
        // Wide, the tab area is always shown: narrowing the window again
        // shows the status column.
        self.tab_view = false;
        self.widening = None;
        egui::Panel::left("status")
            .exact_size(COLUMN_WIDTH)
            .resizable(false)
            .frame(column)
            .show(ui, |ui| self.status_column(ui, Width::Wide));
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(theme::WINDOW))
            .show(ui, |ui| self.tab_area(ui, Width::Wide));
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
        app.pick_splitter = || panic!("the file dialog opened before the answer");
        app.open();
        assert_eq!(app.pending, Some(Pending::Open));
    }

    #[test]
    fn answering_open_shows_the_file_dialog_then_loads_the_file() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.pick_splitter = || Some(PathBuf::from("/other.wasm"));
        app.open();
        app.answer_unsaved(UnsavedAction::Discard);
        // The file has no game yet, so loading it asks which game it is for.
        let path = app.dialog.as_ref().map(|(_, path)| path.clone());
        assert_eq!(path, Some(PathBuf::from("/other.wasm")));
    }

    #[test]
    fn cancelling_open_shows_no_file_dialog() {
        let (_dir, mut app) = app_with_unsaved_edit();
        app.pick_splitter = || panic!("the file dialog opened after Cancel");
        app.open();
        app.answer_unsaved(UnsavedAction::Cancel);
        assert_eq!(app.pending, None);
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
                RunnerEvent::TimerAction(crate::runner::TimerAction::Split),
                Category::AutoSplitter,
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
        ];
        for (event, expected) in cases {
            assert_eq!(server_category(&event), expected, "{event:?}");
        }
    }

    /// The app with its files in a temporary folder, and a few lines in the
    /// log of every category.
    fn app_with_lines() -> (tempfile::TempDir, BridgeApp) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        let app = BridgeApp::with_config(&egui::Context::default(), Some(config));
        (dir, app)
    }

    fn crashed() -> RunnerEvent {
        RunnerEvent::Crashed {
            error: "wasm trap: unreachable".to_owned(),
        }
    }

    fn port_in_use() -> ServerEvent {
        ServerEvent::BindFailed {
            port: 16834,
            error: "Address already in use (os error 98)".to_owned(),
            in_use: true,
        }
    }

    /// The id of the last line logged with `category`.
    fn last_line(app: &BridgeApp, category: Category) -> u64 {
        let all = Filters {
            auto_splitter: true,
            connection: true,
            app: true,
        };
        app.logger
            .view()
            .shown(all)
            .filter(|entry| entry.category == category)
            .last()
            .unwrap()
            .id
    }

    #[test]
    fn show_in_log_after_a_crash_opens_the_crash_with_the_auto_splitter_lines() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(RunnerEvent::AutoSplitterLog("about to crash".to_owned()));
        app.runner_event(crashed());
        let crash = last_line(&app, Category::Error);
        app.show_in_log(Width::Wide);
        assert_eq!(app.tab, Tab::Log);
        assert_eq!(app.highlighted, Some(crash));
        assert!(app.scroll_to_highlighted);
        assert_eq!(
            app.log_filters,
            Filters {
                auto_splitter: true,
                ..Filters::default()
            }
        );
        // The filters are remembered, like any other change to them.
        assert!(app.app_settings.log_filters().auto_splitter);
        assert!(!app.tab_view);
    }

    #[test]
    fn show_in_log_after_a_failed_load_ticks_app_and_runtime() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(RunnerEvent::LoadFailed {
            path: PathBuf::from("foo.wasm"),
            error: "not a valid WebAssembly module".to_owned(),
        });
        let failure = last_line(&app, Category::Error);
        app.show_in_log(Width::Wide);
        assert_eq!(app.highlighted, Some(failure));
        assert_eq!(
            app.log_filters,
            Filters {
                app: true,
                ..Filters::default()
            }
        );
    }

    #[test]
    fn show_in_log_highlights_the_latest_error() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(crashed());
        app.runner_event(RunnerEvent::LoadFailed {
            path: PathBuf::from("foo.wasm"),
            error: "bad".to_owned(),
        });
        let failure = last_line(&app, Category::Error);
        app.show_in_log(Width::Wide);
        assert_eq!(app.highlighted, Some(failure));
    }

    #[test]
    fn port_in_use_has_no_show_in_log() {
        let (_dir, mut app) = app_with_lines();
        app.server_event(port_in_use());
        app.show_in_log(Width::Wide);
        assert_eq!(app.tab, Tab::Connection);
        assert_eq!(app.highlighted, None);
    }

    #[test]
    fn in_the_compact_state_tab_buttons_show_the_tab_view() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(crashed());
        app.show_in_log(Width::Compact);
        assert!(app.tab_view);
        assert_eq!(app.tab, Tab::Log);

        app.tab_view = false;
        app.server_event(port_in_use());
        app.open_tab(Tab::Connection, Width::Compact);
        assert!(app.tab_view);
        assert_eq!(app.tab, Tab::Connection);
    }

    #[test]
    fn show_details_shows_the_tab_view_if_the_window_does_not_widen() {
        let (_dir, mut app) = app_with_lines();
        let ctx = app.ctx.clone();
        app.show_details(&ctx);
        let asked = app.widening.unwrap();

        app.check_widening(&ctx, asked + WIDEN_WAIT / 2);
        assert!(!app.tab_view);
        app.check_widening(&ctx, asked + WIDEN_WAIT);
        assert!(app.tab_view);
        assert_eq!(app.widening, None);
    }

    #[test]
    fn a_trace_pushes_the_next_line_down() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(RunnerEvent::Crashed {
            error: "wasm trap\n    at update (wasm function 42)".to_owned(),
        });
        app.runner_event(RunnerEvent::AutoSplitterLog("after".to_owned()));
        app.log_filters.show(Category::AutoSplitter);
        app.show_in_log(Width::Wide);
        let on_screen = draw_log_tab(&mut app);
        let bottom = |needle: &str| {
            on_screen
                .iter()
                .find(|(text, _)| text.contains(needle))
                .map(|(_, rect)| *rect)
                .unwrap_or_else(|| panic!("{needle} not in {on_screen:?}"))
        };
        let trace = bottom("wasm function 42");
        assert!(trace.top() >= bottom("wasm trap").bottom());
        assert!(bottom("after").top() >= trace.bottom());
    }

    /// Draws the Log tab in an 800 × 600 window, returning the log lines on
    /// screen by their message.
    fn draw_log_tab(app: &mut BridgeApp) -> Vec<(String, egui::Rect)> {
        let ctx = app.ctx.clone();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let input = RawInput {
            screen_rect: Some(screen),
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| app.log_tab(ui));
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_, node)| {
                let bounds = node.bounds()?;
                let rect = egui::Rect::from_min_max(
                    egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                    egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                );
                Some((node.label().or(node.value())?.to_owned(), rect))
            })
            .filter(|(_, rect)| screen.intersects(*rect))
            .collect()
    }

    #[test]
    fn show_in_log_scrolls_to_the_entry() {
        let (_dir, mut app) = app_with_lines();
        app.runner_event(crashed());
        for n in 0..500 {
            app.runner_event(RunnerEvent::AutoSplitterLog(format!("line {n}")));
        }
        // The Log tab follows the newest line, so the crash is out of view.
        app.log_filters.show(Category::AutoSplitter);
        // The first frame measures the lines; the second is at the bottom.
        draw_log_tab(&mut app);
        let on_screen = draw_log_tab(&mut app);
        assert!(
            on_screen.iter().any(|(text, _)| text == "line 499"),
            "{on_screen:?}"
        );
        assert!(!on_screen.iter().any(|(text, _)| text.contains("wasm trap")));

        app.show_in_log(Width::Wide);
        let on_screen = draw_log_tab(&mut app);
        assert!(
            on_screen.iter().any(|(text, _)| text.contains("wasm trap")),
            "{on_screen:?}"
        );
        // Then it stays there rather than jumping back to the newest line.
        let on_screen = draw_log_tab(&mut app);
        assert!(on_screen.iter().any(|(text, _)| text.contains("wasm trap")));
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
