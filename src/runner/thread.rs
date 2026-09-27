//! The Runner thread, which ticks the auto splitter.

use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::{Duration, Instant},
};

use livesplit_auto_splitting::{AutoSplitter, Process, settings};

use super::{RunnerEvent, events::EventSink, timer::BridgeTimer};

/// Instructions for the Runner thread.
pub(super) enum Command {
    /// Run this auto splitter instead of the current one.
    Replace(Box<AutoSplitter<BridgeTimer>>),
    /// Give the running auto splitter these settings.
    SetSettings(settings::Map),
    Unload,
    Shutdown,
}

/// The running auto splitter and what was last reported about it.
struct Active {
    auto_splitter: Box<AutoSplitter<BridgeTimer>>,
    attached: Attached,
    tick_rate: Duration,
}

/// The first attached process's name, if a process is attached: `Some(None)`
/// is a process whose name couldn't be read.
type Attached = Option<Option<String>>;

impl Active {
    fn new(auto_splitter: Box<AutoSplitter<BridgeTimer>>) -> Self {
        let tick_rate = auto_splitter.tick_rate();
        Self {
            auto_splitter,
            attached: None,
            tick_rate,
        }
    }
}

enum Flow {
    Continue,
    Stop,
}

pub(super) fn run(commands: Receiver<Command>, events: EventSink) {
    let mut active: Option<Active> = None;
    let mut next_tick = Instant::now();

    loop {
        let received = match &active {
            None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(_) => commands.recv_timeout(next_tick.saturating_duration_since(Instant::now())),
        };
        match received {
            // New settings take effect on the next tick, which stays on
            // schedule.
            Ok(command @ Command::SetSettings(_)) => {
                apply(command, &mut active, &events);
                continue;
            }
            Ok(command) => {
                if let Flow::Stop = apply(command, &mut active, &events) {
                    return;
                }
                next_tick = Instant::now();
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
        let Some(current) = &mut active else {
            continue;
        };

        let (result, attached) = {
            let mut guard = current.auto_splitter.lock();
            let result = guard.update();
            // Compared as borrowed names, so the name is only copied when it
            // changes, not on every tick.
            let first = guard.attached_processes().next().map(Process::name);
            let changed = first != current.attached.as_ref().map(Option::as_deref);
            let attached = changed.then(|| first.map(|name| name.map(str::to_owned)));
            (result, attached)
        };

        if let Err(error) = result {
            // Reload, unload and shutdown send their command before
            // interrupting, so an interrupted tick finds it waiting. Settings
            // don't interrupt, so they are skipped: the tick really failed.
            let mut waiting = commands.try_recv();
            while let Ok(Command::SetSettings(_)) = waiting {
                waiting = commands.try_recv();
            }
            if let Ok(command) = waiting {
                if let Flow::Stop = apply(command, &mut active, &events) {
                    return;
                }
                next_tick = Instant::now();
                continue;
            }
            let was_attached = current.attached.is_some();
            active = None;
            events.send(RunnerEvent::Crashed {
                error: format!("{error:#}"),
            });
            if was_attached {
                events.send(RunnerEvent::GameDetached);
            }
            continue;
        }

        // Also reported when the first attached process changes, so its name
        // is never stale.
        if let Some(attached) = attached {
            events.send(match &attached {
                Some(process) => RunnerEvent::GameAttached {
                    process: process.clone(),
                },
                None => RunnerEvent::GameDetached,
            });
            current.attached = attached;
        }
        let tick_rate = current.auto_splitter.tick_rate();
        if tick_rate != current.tick_rate {
            current.tick_rate = tick_rate;
            events.send(RunnerEvent::TickRateChanged(tick_rate));
        }
        next_tick = schedule(next_tick, tick_rate, Instant::now());
    }
}

/// Applies a command. The auto splitter it replaces or unloads is dropped.
fn apply(command: Command, active: &mut Option<Active>, events: &EventSink) -> Flow {
    let was_attached = active
        .as_ref()
        .is_some_and(|current| current.attached.is_some());
    let flow = match command {
        Command::Replace(auto_splitter) => {
            *active = Some(Active::new(auto_splitter));
            Flow::Continue
        }
        Command::SetSettings(settings) => {
            if let Some(current) = active {
                current.auto_splitter.set_settings_map(settings);
            }
            // The same auto splitter keeps running, still attached.
            return Flow::Continue;
        }
        Command::Unload => {
            *active = None;
            Flow::Continue
        }
        Command::Shutdown => {
            *active = None;
            Flow::Stop
        }
    };
    if was_attached {
        events.send(RunnerEvent::GameDetached);
    }
    flow
}

/// When to tick next: one tick rate after the previous tick, or now if that
/// has already passed, so a slow tick doesn't cause a burst of catch-up ticks.
fn schedule(previous: Instant, tick_rate: Duration, now: Instant) -> Instant {
    match previous.checked_add(tick_rate) {
        Some(next) if next > now => next,
        _ => now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_tick_is_one_tick_rate_later() {
        let start = Instant::now();
        let rate = Duration::from_millis(10);
        assert_eq!(schedule(start, rate, start), start + rate);
    }

    #[test]
    fn late_tick_runs_now_without_catching_up() {
        let start = Instant::now();
        let now = start + Duration::from_millis(50);
        assert_eq!(schedule(start, Duration::from_millis(10), now), now);
    }

    #[test]
    fn huge_tick_rate_does_not_overflow() {
        let start = Instant::now();
        assert_eq!(schedule(start, Duration::MAX, start), start);
    }
}
