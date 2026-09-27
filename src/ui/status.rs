//! What the status column shows, worked out from the Runner's events, and
//! the error card's error, from the Runner's and the Server's.

use std::{borrow::Cow, path::Path, time::Duration};

use crate::{
    logging::Category,
    runner::{RunnerEvent, file_name},
    server::ServerEvent,
};

/// The error card's text for a crash.
const CRASHED_MESSAGE: &str =
    "The auto splitter stopped because of an error. Press Reload to start it again.";

/// The state the Game card shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    /// No auto splitter is loaded.
    None,
    /// The auto splitter runs but no game process is attached. This is
    /// normal, not an error.
    Waiting,
    /// A game process is attached.
    Attached,
    /// The auto splitter crashed and isn't running, until it is loaded again.
    Stopped,
}

/// The error the error card shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusError {
    /// The auto splitter crashed. The card offers Reload.
    Crashed,
    /// A file couldn't be loaded.
    LoadFailed { message: String },
    /// The server couldn't listen because another program has the port. The
    /// card offers Open Connection.
    PortInUse { port: u16 },
    /// The server couldn't listen for another reason. The card offers Open
    /// Connection.
    ServerFailed { message: String },
}

impl StatusError {
    /// The text of the error card.
    pub fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Crashed => CRASHED_MESSAGE.into(),
            Self::LoadFailed { message } | Self::ServerFailed { message } => message.into(),
            Self::PortInUse { port } => format!(
                "Port {port} is already in use. Choose another port in Connection and restart the server."
            )
            .into(),
        }
    }

    /// The log category ticked by Show in log, for the errors that need
    /// more detail than the card has: the auto splitter's own messages
    /// before a crash, and the runtime's messages about a failed load.
    /// `None` for the errors without Show in log.
    pub fn log_category(&self) -> Option<Category> {
        match self {
            Self::Crashed => Some(Category::AutoSplitter),
            Self::LoadFailed { .. } => Some(Category::App),
            Self::PortInUse { .. } | Self::ServerFailed { .. } => None,
        }
    }

    /// Whether the Server reported this error, rather than the Runner.
    pub fn is_server(&self) -> bool {
        matches!(self, Self::PortInUse { .. } | Self::ServerFailed { .. })
    }
}

/// The status of the auto splitter and the game, updated from each
/// [`RunnerEvent`].
#[derive(Debug, Default)]
pub struct Status {
    loaded: bool,
    crashed: bool,
    /// The attached process's name, if a process is attached: `Some(None)` is
    /// a process whose name couldn't be read.
    process: Option<Option<String>>,
    tick_rate: Option<Duration>,
    error: Option<StatusError>,
}

impl Status {
    /// Updates the status with an event.
    pub fn apply(&mut self, event: &RunnerEvent) {
        match event {
            RunnerEvent::Loaded { tick_rate, .. } => {
                self.loaded = true;
                self.crashed = false;
                self.process = None;
                self.tick_rate = Some(*tick_rate);
                // Whatever an auto splitter error described, a crash or an
                // earlier failed load, is over: the auto splitter now
                // running is the one just loaded. A server error isn't.
                if self.error.as_ref().is_some_and(|error| !error.is_server()) {
                    self.error = None;
                }
            }
            RunnerEvent::LoadFailed { path, error } => {
                self.error = Some(StatusError::LoadFailed {
                    // After a crash the old auto splitter is loaded but not
                    // running, so the card mustn't say it still runs.
                    message: load_failed_message(path, error, self.loaded && !self.crashed),
                });
            }
            RunnerEvent::Unloaded => {
                *self = Self {
                    error: self.error.take(),
                    ..Self::default()
                }
            }
            RunnerEvent::Crashed { .. } => {
                self.crashed = true;
                self.process = None;
                self.error = Some(StatusError::Crashed);
            }
            RunnerEvent::GameAttached { process } => self.process = Some(process.clone()),
            RunnerEvent::GameDetached => self.process = None,
            RunnerEvent::TickRateChanged(tick_rate) => self.tick_rate = Some(*tick_rate),
            RunnerEvent::AutoSplitterLog(_)
            | RunnerEvent::RuntimeLog { .. }
            | RunnerEvent::TimerAction { .. }
            | RunnerEvent::SettingsWidgets(_) => {}
        }
    }

    /// Updates the error with a Server event: a failure to listen replaces
    /// it, and listening again clears a server error.
    pub fn apply_server(&mut self, event: &ServerEvent) {
        match event {
            ServerEvent::Listening { .. } => {
                if self.error.as_ref().is_some_and(StatusError::is_server) {
                    self.error = None;
                }
            }
            ServerEvent::BindFailed {
                port, in_use: true, ..
            } => self.error = Some(StatusError::PortInUse { port: *port }),
            ServerEvent::BindFailed { port, error, .. } => {
                self.error = Some(StatusError::ServerFailed {
                    message: server_failed_message(*port, error),
                });
            }
            ServerEvent::TimerConnected { .. }
            | ServerEvent::TimerDisconnected { .. }
            | ServerEvent::HandshakeFailed { .. }
            | ServerEvent::CommandRejected { .. } => {}
        }
    }

    /// The state the Game card shows.
    pub fn game(&self) -> GameState {
        if !self.loaded {
            GameState::None
        } else if self.crashed {
            GameState::Stopped
        } else if self.process.is_some() {
            GameState::Attached
        } else {
            GameState::Waiting
        }
    }

    /// The Game card's detail line while attached: the process name and the
    /// tick rate, or in the compact state only the process name. Without a
    /// process name, the tick rate stands in, so the line is never empty.
    pub fn game_detail(&self, compact: bool) -> Option<String> {
        let process = self.process.clone().flatten();
        let tick_rate = self.tick_rate.and_then(format_tick_rate);
        match (process, tick_rate) {
            (Some(process), Some(tick_rate)) if !compact => {
                Some(format!("{process} · {tick_rate}"))
            }
            (Some(process), _) => Some(process),
            (None, tick_rate) => tick_rate,
        }
    }

    /// The error to show, if any.
    pub fn error(&self) -> Option<&StatusError> {
        self.error.as_ref()
    }

    /// Hides the error, until the next one.
    pub fn dismiss_error(&mut self) {
        self.error = None;
    }
}

/// A tick interval as a rate, like "20 Hz", or "0.5 Hz" when not whole.
/// `None` for a zero interval, which has no rate.
pub fn format_tick_rate(interval: Duration) -> Option<String> {
    if interval.is_zero() {
        return None;
    }
    let hertz = (10.0 / interval.as_secs_f64()).round() / 10.0;
    Some(if hertz.fract() == 0.0 {
        format!("{hertz:.0} Hz")
    } else {
        format!("{hertz:.1} Hz")
    })
}

/// The error card's text for a file that couldn't be loaded. It keeps only
/// the top-level reason; the full error is in the log.
fn load_failed_message(path: &Path, error: &str, previous_running: bool) -> String {
    let name = file_name(path);
    let reason = short_reason(error);
    let mut message = if reason.is_empty() {
        format!("Couldn't load {name}.")
    } else {
        format!("Couldn't load {name}: {reason}.")
    };
    if previous_running {
        message.push_str(" The previous auto splitter is still running.");
    }
    message
}

/// The error card's text for a server that couldn't listen, for a reason
/// other than the port being in use.
fn server_failed_message(port: u16, error: &str) -> String {
    let reason = io_reason(error);
    if reason.is_empty() {
        format!("Couldn't start the server on port {port}.")
    } else {
        format!("Couldn't start the server on port {port}: {reason}.")
    }
}

/// An I/O error's message without the OS error number, like "permission
/// denied" from "Permission denied (os error 13)".
fn io_reason(error: &str) -> String {
    let error = match error.rfind(" (os error ") {
        Some(start) if error.ends_with(')') => &error[..start],
        _ => error,
    };
    short_reason(error)
}

/// The top-level message of an error the Runner reported. The Runner joins an
/// error's causes with ": ", so it is the text before the first one.
fn short_reason(error: &str) -> String {
    let reason = error
        .split(": ")
        .next()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('.');
    // Upstream messages start with a capital, which reads oddly after a
    // colon. An acronym such as "WASI" keeps its case.
    let mut chars = reason.chars();
    match (chars.next(), chars.next()) {
        (Some(first), Some(second)) if first.is_uppercase() && !second.is_uppercase() => {
            let mut text: String = first.to_lowercase().collect();
            text.push_str(&reason[first.len_utf8()..]);
            text
        }
        _ => reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn loaded(millis: u64) -> RunnerEvent {
        RunnerEvent::Loaded {
            path: PathBuf::from("/splitters/gta.wasm"),
            tick_rate: Duration::from_millis(millis),
        }
    }

    fn load_failed(error: &str) -> RunnerEvent {
        RunnerEvent::LoadFailed {
            path: PathBuf::from("/splitters/foo.wasm"),
            error: error.to_owned(),
        }
    }

    fn crashed() -> RunnerEvent {
        RunnerEvent::Crashed {
            error: "wasm trap".to_owned(),
        }
    }

    fn attached(process: Option<&str>) -> RunnerEvent {
        RunnerEvent::GameAttached {
            process: process.map(str::to_owned),
        }
    }

    fn status(events: &[RunnerEvent]) -> Status {
        let mut status = Status::default();
        for event in events {
            status.apply(event);
        }
        status
    }

    #[test]
    fn nothing_loaded_at_first() {
        let status = Status::default();
        assert_eq!(status.game(), GameState::None);
        assert_eq!(status.error(), None);
    }

    #[test]
    fn waits_for_the_game_once_loaded() {
        assert_eq!(status(&[loaded(50)]).game(), GameState::Waiting);
    }

    #[test]
    fn attaches_and_detaches() {
        let mut status = status(&[loaded(50), attached(Some("SanAndreas.exe"))]);
        assert_eq!(status.game(), GameState::Attached);
        status.apply(&RunnerEvent::GameDetached);
        assert_eq!(status.game(), GameState::Waiting);
    }

    #[test]
    fn crash_stops_until_the_next_load() {
        let mut status = status(&[loaded(50), attached(Some("SanAndreas.exe")), crashed()]);
        assert_eq!(status.game(), GameState::Stopped);
        status.apply(&RunnerEvent::GameDetached);
        assert_eq!(status.game(), GameState::Stopped);
        status.apply(&loaded(50));
        assert_eq!(status.game(), GameState::Waiting);
    }

    #[test]
    fn failed_load_after_a_crash_stays_stopped() {
        let status = status(&[loaded(50), crashed(), load_failed("bad")]);
        assert_eq!(status.game(), GameState::Stopped);
        assert_eq!(
            status.error().unwrap().message(),
            "Couldn't load foo.wasm: bad."
        );
    }

    #[test]
    fn unload_shows_nothing_loaded() {
        let status = status(&[loaded(50), crashed(), RunnerEvent::Unloaded]);
        assert_eq!(status.game(), GameState::None);
    }

    #[test]
    fn a_new_load_starts_unattached() {
        let status = status(&[loaded(50), attached(Some("SanAndreas.exe")), loaded(50)]);
        assert_eq!(status.game(), GameState::Waiting);
    }

    #[test]
    fn detail_has_process_and_tick_rate_when_wide() {
        let status = status(&[loaded(50), attached(Some("SanAndreas.exe"))]);
        assert_eq!(
            status.game_detail(false).as_deref(),
            Some("SanAndreas.exe · 20 Hz")
        );
    }

    #[test]
    fn detail_has_only_the_process_when_compact() {
        let status = status(&[loaded(50), attached(Some("SanAndreas.exe"))]);
        assert_eq!(status.game_detail(true).as_deref(), Some("SanAndreas.exe"));
    }

    #[test]
    fn detail_without_a_process_name_is_the_tick_rate() {
        let status = status(&[loaded(50), attached(None)]);
        assert_eq!(status.game_detail(false).as_deref(), Some("20 Hz"));
        assert_eq!(status.game_detail(true).as_deref(), Some("20 Hz"));
    }

    #[test]
    fn detail_follows_tick_rate_changes() {
        let status = status(&[
            loaded(50),
            attached(Some("SanAndreas.exe")),
            RunnerEvent::TickRateChanged(Duration::from_secs(2)),
        ]);
        assert_eq!(
            status.game_detail(false).as_deref(),
            Some("SanAndreas.exe · 0.5 Hz")
        );
    }

    #[test]
    fn tick_rates_are_whole_or_one_decimal() {
        let rate = |secs: f64| format_tick_rate(Duration::from_secs_f64(secs));
        assert_eq!(rate(1.0 / 20.0).as_deref(), Some("20 Hz"));
        assert_eq!(rate(1.0 / 120.0).as_deref(), Some("120 Hz"));
        assert_eq!(rate(1.0 / 60.0).as_deref(), Some("60 Hz"));
        assert_eq!(rate(2.0).as_deref(), Some("0.5 Hz"));
        assert_eq!(rate(0.3).as_deref(), Some("3.3 Hz"));
        assert_eq!(format_tick_rate(Duration::ZERO), None);
    }

    #[test]
    fn crash_shows_the_crash_error() {
        let status = status(&[loaded(50), crashed()]);
        assert_eq!(status.error(), Some(&StatusError::Crashed));
        assert_eq!(
            status.error().unwrap().message(),
            "The auto splitter stopped because of an error. Press Reload to start it again."
        );
    }

    #[test]
    fn load_failed_with_nothing_loaded() {
        let status = status(&[load_failed("couldn't read the file: No such file")]);
        assert_eq!(
            status.error().unwrap().message(),
            "Couldn't load foo.wasm: couldn't read the file."
        );
    }

    #[test]
    fn load_failed_while_another_runs() {
        let status = status(&[
            loaded(50),
            load_failed("Failed loading the WebAssembly module.: bad magic number"),
        ]);
        assert_eq!(
            status.error().unwrap().message(),
            "Couldn't load foo.wasm: failed loading the WebAssembly module. \
             The previous auto splitter is still running."
        );
    }

    #[test]
    fn only_the_latest_error_is_kept() {
        let status = status(&[loaded(50), crashed(), load_failed("bad")]);
        assert!(matches!(
            status.error(),
            Some(StatusError::LoadFailed { .. })
        ));
    }

    #[test]
    fn dismiss_hides_the_error() {
        let mut status = status(&[loaded(50), crashed()]);
        status.dismiss_error();
        assert_eq!(status.error(), None);
        assert_eq!(status.game(), GameState::Stopped);
    }

    #[test]
    fn a_successful_load_clears_the_error() {
        let status = status(&[loaded(50), crashed(), loaded(50)]);
        assert_eq!(status.error(), None);
    }

    fn port_in_use() -> ServerEvent {
        ServerEvent::BindFailed {
            port: 16834,
            error: "Address already in use (os error 98)".to_owned(),
            in_use: true,
        }
    }

    fn listening() -> ServerEvent {
        ServerEvent::Listening { port: 16834 }
    }

    #[test]
    fn port_in_use_shows_its_error() {
        let mut status = Status::default();
        status.apply_server(&port_in_use());
        let error = status.error().unwrap();
        assert_eq!(error, &StatusError::PortInUse { port: 16834 });
        assert!(error.is_server());
        assert_eq!(
            error.message(),
            "Port 16834 is already in use. Choose another port in Connection and restart the server."
        );
    }

    #[test]
    fn other_bind_failures_give_a_short_reason() {
        let mut status = Status::default();
        status.apply_server(&ServerEvent::BindFailed {
            port: 80,
            error: "Permission denied (os error 13)".to_owned(),
            in_use: false,
        });
        let error = status.error().unwrap();
        assert!(error.is_server());
        assert_eq!(
            error.message(),
            "Couldn't start the server on port 80: permission denied."
        );
    }

    #[test]
    fn loading_an_auto_splitter_keeps_a_server_error() {
        let mut status = Status::default();
        status.apply_server(&port_in_use());
        status.apply(&loaded(50));
        assert_eq!(
            status.error(),
            Some(&StatusError::PortInUse { port: 16834 })
        );
    }

    #[test]
    fn listening_clears_a_server_error() {
        let mut status = Status::default();
        status.apply_server(&port_in_use());
        status.apply_server(&listening());
        assert_eq!(status.error(), None);
    }

    #[test]
    fn listening_keeps_an_auto_splitter_error() {
        let mut status = status(&[loaded(50), crashed()]);
        status.apply_server(&listening());
        assert_eq!(status.error(), Some(&StatusError::Crashed));
    }

    #[test]
    fn a_server_error_replaces_an_auto_splitter_error_and_back() {
        let mut status = status(&[loaded(50), crashed()]);
        status.apply_server(&port_in_use());
        assert_eq!(
            status.error(),
            Some(&StatusError::PortInUse { port: 16834 })
        );
        status.apply(&crashed());
        assert_eq!(status.error(), Some(&StatusError::Crashed));
    }

    #[test]
    fn timer_events_leave_the_error_alone() {
        let mut status = Status::default();
        status.apply_server(&port_in_use());
        let address = "192.168.1.42:53122".parse().unwrap();
        status.apply_server(&ServerEvent::TimerConnected { id: 1, address });
        status.apply_server(&ServerEvent::TimerDisconnected { id: 1, address });
        assert_eq!(
            status.error(),
            Some(&StatusError::PortInUse { port: 16834 })
        );
    }

    #[test]
    fn io_reason_drops_the_os_error_number() {
        assert_eq!(
            io_reason("Permission denied (os error 13)"),
            "permission denied"
        );
        assert_eq!(io_reason("couldn't start"), "couldn't start");
        assert_eq!(
            server_failed_message(1, ""),
            "Couldn't start the server on port 1."
        );
    }

    #[test]
    fn short_reason_is_the_top_level_message() {
        assert_eq!(
            short_reason("Failed compiling: invalid: x"),
            "failed compiling"
        );
        assert_eq!(short_reason("WASI failed: x"), "WASI failed");
        assert_eq!(short_reason("no causes"), "no causes");
        assert_eq!(short_reason(""), "");
    }

    #[test]
    fn empty_reason_still_names_the_file() {
        assert_eq!(
            load_failed_message(Path::new("foo.wasm"), "", false),
            "Couldn't load foo.wasm."
        );
    }
}
