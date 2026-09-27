//! The connection between the auto splitter and the timer it controls.

use livesplit_auto_splitting::{TimerState, time};

/// An action the auto splitter takes on the timer.
#[derive(Debug, Clone, PartialEq)]
pub enum TimerAction {
    Start,
    Split,
    SkipSplit,
    UndoSplit,
    Reset,
    SetGameTime(time::Duration),
    PauseGameTime,
    ResumeGameTime,
    SetVariable { key: String, value: String },
}

impl std::fmt::Display for TimerAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => f.write_str("start"),
            Self::Split => f.write_str("split"),
            Self::SkipSplit => f.write_str("skip split"),
            Self::UndoSplit => f.write_str("undo split"),
            Self::Reset => f.write_str("reset"),
            Self::SetGameTime(time) => write!(f, "set game time to {time}"),
            Self::PauseGameTime => f.write_str("pause game time"),
            Self::ResumeGameTime => f.write_str("resume game time"),
            Self::SetVariable { key, value } => write!(f, "set variable {key} to {value}"),
        }
    }
}

/// The timer the auto splitter controls: actions go to it, and state queries
/// are answered from it. The Server implements this to control LiveSplit One.
/// State queries are called on every tick, so they must answer immediately.
pub trait TimerLink: Send + Sync + 'static {
    fn state(&self) -> TimerState;
    fn current_split_index(&self) -> Option<usize>;
    fn segment_splitted(&self, index: usize) -> Option<bool>;
    /// Sends the action to the timers, returning how many it was sent to: 0
    /// when it was dropped. Never blocks.
    fn send(&self, action: TimerAction) -> usize;
}

/// No timer at all: it is never running, and actions go nowhere.
#[cfg(test)]
pub struct NoTimer;

#[cfg(test)]
impl TimerLink for NoTimer {
    fn state(&self) -> TimerState {
        TimerState::NotRunning
    }

    fn current_split_index(&self) -> Option<usize> {
        None
    }

    fn segment_splitted(&self, _index: usize) -> Option<bool> {
        None
    }

    fn send(&self, _action: TimerAction) -> usize {
        0
    }
}
