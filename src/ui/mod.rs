//! The application window: the status column and, when wide, the tab
//! area.

mod components;
mod connection_tab;
mod folder;
mod server_status;
mod status;
mod theme;

use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, mpsc::Receiver},
    time::{Duration, Instant},
};

use eframe::egui::{self, Frame, Margin, ScrollArea};

use crate::{
    config::{self, AppSettings, Config, Game, GameSummary, Splitters},
    logging::{self, Category, Filters, Logger},
    runner::{NoTimer, Runner, RunnerEvent, file_name},
    server::{self, NetworkAddress, Server, ServerEvent},
    version::VERSION,
};
use components::{
    ErrorAction, GameChoice, GameDialog, GameDialogAction, GameDialogKind, LOG_LINE_HEIGHT,
    LogAction, SplitterAction, Width,
};
use connection_tab::ConnectionTab;
use server_status::ServerStatus;
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

/// The tabs of the tab area. Settings and Preferences come with #8 and #13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Connection,
    Log,
}

/// The application. Later issues add the Last action card and the other
/// tabs.
pub struct BridgeApp {
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
    /// The log entry reached through Show in log (#14), highlighted.
    highlighted: Option<u64>,
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
}

impl BridgeApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let wake = {
            let ctx = ctx.clone();
            move || ctx.request_repaint()
        };
        // The Server replaces NoTimer (#10, #11).
        let (runner, events) = Runner::new(Arc::new(NoTimer), wake.clone());
        let config = Config::standard();
        let loaded = config.as_ref().map(AppSettings::load);
        let app_settings = match &loaded {
            Some(Ok(settings)) => settings.clone(),
            _ => AppSettings::default(),
        };
        let port = app_settings.port();
        let (server, server_events) = Server::new(port, wake);
        let mut app = Self {
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
            config,
            app_settings,
            loading: HashMap::new(),
            game: None,
            dialog: None,
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
            match &event {
                RunnerEvent::Loaded { path, .. } => self.game = self.loading.remove(path),
                RunnerEvent::LoadFailed { path, .. } => {
                    self.loading.remove(path);
                }
                RunnerEvent::Unloaded => self.game = None,
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
            components::app_header(ui, width);

            if let Some(error) = self.status.error() {
                match components::error_card(ui, error, width) {
                    Some(ErrorAction::Reload) => self.reload(),
                    Some(ErrorAction::OpenConnection) => self.tab = Tab::Connection,
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
                Some(SplitterAction::Reload) => self.reload(),
                Some(SplitterAction::Change) => self.change_game(),
                None => {}
            }

            let detail = self.status.game_detail(width == Width::Compact);
            components::game_card(ui, &self.status.game(), detail.as_deref(), width);

            let urls = self.server_status.urls(&self.addresses);
            components::timer_card(ui, self.server_status.timer_state(), &urls, width);
        });
    }

    /// The tab area: the tab strip, then the active tab.
    fn tab_area(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 0.0;
        components::tab_strip(
            ui,
            &[(Tab::Connection, "Connection"), (Tab::Log, "Log")],
            &mut self.tab,
        );
        Frame::NONE
            .inner_margin(Margin::same(24))
            .show(ui, |ui| match self.tab {
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

    /// Asks for a `.wasm` file and loads it.
    fn open(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Auto splitter", &["wasm"])
            .pick_file()
        {
            self.load(path);
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

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.handle_events();
        self.game_dialog(&ui.ctx().clone());
        self.refresh_addresses(ui.ctx());
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
            .frame(Frame::NONE.fill(theme::WINDOW))
            .show(ui, |ui| self.tab_area(ui));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
