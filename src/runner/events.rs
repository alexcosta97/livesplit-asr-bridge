//! What the Runner reports.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
    time::Duration,
};

use livesplit_auto_splitting::settings::Widget;

use super::TimerAction;

/// Events are dropped when this many are waiting, so a chatty auto splitter
/// can't use unbounded memory while the window isn't reading them.
const CAPACITY: usize = 10_000;

/// Something that happened in the Runner.
#[derive(Debug, Clone, PartialEq)]
pub enum RunnerEvent {
    /// An auto splitter was loaded and started. `tick_rate` is how long it
    /// waits between ticks at first; later changes are `TickRateChanged`.
    Loaded { path: PathBuf, tick_rate: Duration },
    /// A file couldn't be loaded. The previous auto splitter keeps running.
    LoadFailed { path: PathBuf, error: String },
    /// The auto splitter was unloaded.
    Unloaded,
    /// The auto splitter crashed, and the runtime stopped it.
    Crashed { error: String },
    /// The auto splitter attached to a game process, having had none, or the
    /// first attached process changed. `process` is its executable's file
    /// name, if it could be read.
    GameAttached { process: Option<String> },
    /// The auto splitter no longer has any game process attached.
    GameDetached,
    /// The auto splitter changed how long it waits between ticks.
    TickRateChanged(Duration),
    /// A message the auto splitter printed.
    AutoSplitterLog(String),
    /// A message from the runtime about the auto splitter.
    RuntimeLog { level: LogLevel, message: String },
    /// The auto splitter took an action on the timer, which was sent to
    /// `sent_to` timers: 0 when no timer was connected, so it was dropped.
    /// A split carries the name of the segment it split, when the timer gave
    /// it.
    TimerAction {
        action: TimerAction,
        sent_to: usize,
        segment: Option<String>,
    },
    /// The auto splitter published its settings widgets, or changed them.
    SettingsWidgets(Widgets),
}

/// The settings widgets an auto splitter publishes, in order. Two are equal
/// when they are the same list the runtime published, not merely equal
/// widgets.
#[derive(Clone, Default)]
pub struct Widgets(pub Arc<Vec<Widget>>);

impl PartialEq for Widgets {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for Widgets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|widget| &*widget.key))
            .finish()
    }
}

impl RunnerEvent {
    /// A one-line description for people.
    pub fn describe(&self) -> String {
        match self {
            Self::Loaded { path, .. } => format!("Loaded {}", file_name(path)),
            Self::LoadFailed { path, error } => {
                format!("Couldn't load {}: {error}", file_name(path))
            }
            Self::Unloaded => "Unloaded the auto splitter".to_owned(),
            Self::Crashed { error } => format!(
                "The auto splitter stopped because of an error. Press Reload to start it again. {error}"
            ),
            Self::GameAttached {
                process: Some(name),
            } => format!("Attached to the game ({name})"),
            Self::GameAttached { process: None } => "Attached to the game".to_owned(),
            Self::GameDetached => "Detached from the game".to_owned(),
            Self::TickRateChanged(interval) => {
                format!(
                    "Tick rate changed to {:.1} Hz",
                    1.0 / interval.as_secs_f64()
                )
            }
            Self::AutoSplitterLog(message) => message.clone(),
            Self::RuntimeLog { level, message } => format!("{level:?}: {message}"),
            Self::TimerAction {
                action, sent_to: 0, ..
            } => {
                format!("Dropped {action}: no timer connected")
            }
            Self::TimerAction {
                action, sent_to: 1, ..
            } => format!("Sent {action} to 1 timer"),
            Self::TimerAction {
                action, sent_to, ..
            } => format!("Sent {action} to {sent_to} timers"),
            Self::SettingsWidgets(widgets) => match widgets.0.len() {
                1 => "The auto splitter published 1 setting".to_owned(),
                n => format!("The auto splitter published {n} settings"),
            },
        }
    }

    /// Whether this is an error to show to people.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::LoadFailed { .. } | Self::Crashed { .. })
    }
}

/// The file name of `path`, or the whole path if it has none.
pub fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// How important a runtime message is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl From<livesplit_auto_splitting::LogLevel> for LogLevel {
    fn from(level: livesplit_auto_splitting::LogLevel) -> Self {
        use livesplit_auto_splitting::LogLevel as L;
        match level {
            L::Trace => Self::Trace,
            L::Debug => Self::Debug,
            L::Info => Self::Info,
            L::Warning => Self::Warning,
            L::Error => Self::Error,
        }
    }
}

/// Sends events to the receiver, then wakes it.
#[derive(Clone)]
pub(super) struct EventSink {
    sender: SyncSender<RunnerEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink {
    pub(super) fn new(wake: impl Fn() + Send + Sync + 'static) -> (Self, Receiver<RunnerEvent>) {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let sink = Self {
            sender,
            wake: Arc::new(wake),
        };
        (sink, receiver)
    }

    pub(super) fn send(&self, event: RunnerEvent) {
        // A full or closed channel drops the event: nobody is reading.
        if self.sender.try_send(event).is_ok() {
            (self.wake)();
        }
    }
}
