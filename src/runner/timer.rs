//! The timer the runtime gives the auto splitter.

use std::{fmt, sync::Arc};

use livesplit_auto_splitting::{LogLevel, Timer, TimerState, time};

use super::{RunnerEvent, TimerAction, TimerLink, events::EventSink};

/// Forwards the auto splitter's actions to the [`TimerLink`] and reports them,
/// with how many timers they were sent to, as events; answers state queries
/// from the link.
pub(super) struct BridgeTimer {
    link: Arc<dyn TimerLink>,
    events: EventSink,
}

impl BridgeTimer {
    pub(super) fn new(link: Arc<dyn TimerLink>, events: EventSink) -> Self {
        Self { link, events }
    }

    fn act(&self, action: TimerAction) {
        let sent_to = self.link.send(action.clone());
        self.events
            .send(RunnerEvent::TimerAction { action, sent_to });
    }
}

impl Timer for BridgeTimer {
    fn state(&self) -> TimerState {
        self.link.state()
    }

    fn current_split_index(&self) -> Option<usize> {
        self.link.current_split_index()
    }

    fn segment_splitted(&self, idx: usize) -> Option<bool> {
        self.link.segment_splitted(idx)
    }

    fn start(&mut self) {
        self.act(TimerAction::Start);
    }

    fn split(&mut self) {
        self.act(TimerAction::Split);
    }

    fn skip_split(&mut self) {
        self.act(TimerAction::SkipSplit);
    }

    fn undo_split(&mut self) {
        self.act(TimerAction::UndoSplit);
    }

    fn reset(&mut self) {
        self.act(TimerAction::Reset);
    }

    fn set_game_time(&mut self, time: time::Duration) {
        self.act(TimerAction::SetGameTime(time));
    }

    fn pause_game_time(&mut self) {
        self.act(TimerAction::PauseGameTime);
    }

    fn resume_game_time(&mut self) {
        self.act(TimerAction::ResumeGameTime);
    }

    fn set_variable(&mut self, key: &str, value: &str) {
        self.act(TimerAction::SetVariable {
            key: key.to_owned(),
            value: value.to_owned(),
        });
    }

    fn log_auto_splitter(&mut self, message: fmt::Arguments) {
        self.events
            .send(RunnerEvent::AutoSplitterLog(message.to_string()));
    }

    fn log_runtime(&mut self, message: fmt::Arguments, log_level: LogLevel) {
        self.events.send(RunnerEvent::RuntimeLog {
            level: log_level.into(),
            message: message.to_string(),
        });
    }
}
