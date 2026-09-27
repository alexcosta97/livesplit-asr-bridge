//! What the Last action card shows (spec §6.2): the auto splitter's latest
//! timer actions in this session, worked out from the Runner's events.

use std::{collections::VecDeque, time::Instant};

use chrono::NaiveTime;
use livesplit_auto_splitting::time;

use crate::runner::{RunnerEvent, TimerAction};

/// The latest action and the 2 before it.
const KEPT: usize = 3;

/// An action on the Last action card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownAction {
    /// What the auto splitter did, like "Split" or "Game time 1:23:45.600".
    pub word: String,
    /// The time of day it happened.
    pub time: NaiveTime,
    /// Whether it was sent to a timer: it wasn't when none was connected.
    pub sent: bool,
    /// When it arrived, for the card's flash.
    pub arrived: Instant,
    /// Whether it sets the game time, so the next game time replaces it.
    game_time: bool,
}

/// The actions on the Last action card, newest first. Only this session's;
/// they are not stored.
#[derive(Debug, Default)]
pub struct LastActions {
    shown: VecDeque<ShownAction>,
}

impl LastActions {
    /// Adds the action an event reports, at `time` of day and `now`. Custom
    /// variables are logged but not shown. Auto splitters often set the game
    /// time on every tick, so a game time right after another replaces it,
    /// rather than push the other actions off the card; the card flashes
    /// only for the first.
    pub fn apply(&mut self, event: &RunnerEvent, time: NaiveTime, now: Instant) {
        let RunnerEvent::TimerAction { action, sent_to } = event else {
            return;
        };
        let Some(word) = word(action) else {
            return;
        };
        let game_time = matches!(action, TimerAction::SetGameTime(_));
        let sent = *sent_to > 0;
        if let Some(latest) = self.shown.front_mut()
            && game_time
            && latest.game_time
        {
            latest.word = word;
            latest.time = time;
            latest.sent = sent;
            return;
        }
        self.shown.push_front(ShownAction {
            word,
            time,
            sent,
            arrived: now,
            game_time,
        });
        self.shown.truncate(KEPT);
    }

    /// The latest action, if there has been one.
    pub fn latest(&self) -> Option<&ShownAction> {
        self.shown.front()
    }

    /// The actions before the latest, newest first.
    pub fn previous(&self) -> impl Iterator<Item = &ShownAction> {
        self.shown.iter().skip(1)
    }
}

/// What the card says for an action, or `None` for one it doesn't show.
/// Splits carry no segment name: the app never makes one up (spec §6.2).
fn word(action: &TimerAction) -> Option<String> {
    Some(match action {
        TimerAction::Start => "Start".to_owned(),
        TimerAction::Split => "Split".to_owned(),
        TimerAction::SkipSplit => "Skip split".to_owned(),
        TimerAction::UndoSplit => "Undo split".to_owned(),
        TimerAction::Reset => "Reset".to_owned(),
        TimerAction::SetGameTime(time) => format!("Game time {}", game_time(*time)),
        TimerAction::PauseGameTime => "Pause game time".to_owned(),
        TimerAction::ResumeGameTime => "Resume game time".to_owned(),
        TimerAction::SetVariable { .. } => return None,
    })
}

/// A game time to the millisecond, like "1:23:45.600", or "3:07.250" under
/// an hour.
fn game_time(time: time::Duration) -> String {
    let sign = if time.is_negative() { "-" } else { "" };
    let millis = time.whole_milliseconds().unsigned_abs();
    let (hours, minutes) = (millis / 3_600_000, millis / 60_000 % 60);
    let (seconds, millis) = (millis / 1000 % 60, millis % 1000);
    if hours > 0 {
        format!("{sign}{hours}:{minutes:02}:{seconds:02}.{millis:03}")
    } else {
        format!("{sign}{minutes}:{seconds:02}.{millis:03}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u32) -> NaiveTime {
        NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0).unwrap()
    }

    fn action(action: TimerAction, sent_to: usize) -> RunnerEvent {
        RunnerEvent::TimerAction { action, sent_to }
    }

    #[test]
    fn nothing_is_shown_before_the_first_action() {
        let actions = LastActions::default();
        assert_eq!(actions.latest(), None);
        assert_eq!(actions.previous().count(), 0);
    }

    #[test]
    fn keeps_the_latest_action_and_the_two_before_it() {
        let mut actions = LastActions::default();
        let now = Instant::now();
        for (index, event) in [
            action(TimerAction::Reset, 1),
            action(TimerAction::Start, 1),
            action(TimerAction::Split, 1),
            action(TimerAction::SkipSplit, 1),
        ]
        .iter()
        .enumerate()
        {
            actions.apply(event, at(index as u32), now);
        }
        let latest = actions.latest().unwrap();
        assert_eq!((latest.word.as_str(), latest.time), ("Skip split", at(3)));
        let previous: Vec<_> = actions.previous().map(|a| a.word.as_str()).collect();
        assert_eq!(previous, ["Split", "Start"]);
    }

    #[test]
    fn game_times_in_a_row_replace_each_other() {
        let mut actions = LastActions::default();
        let first = Instant::now();
        let game_time = |seconds| {
            action(
                TimerAction::SetGameTime(time::Duration::seconds(seconds)),
                1,
            )
        };
        actions.apply(&action(TimerAction::Start, 1), at(0), first);
        actions.apply(&game_time(1), at(1), first);
        actions.apply(
            &game_time(2),
            at(2),
            first + std::time::Duration::from_secs(1),
        );
        let latest = actions.latest().unwrap();
        assert_eq!(
            (latest.word.as_str(), latest.time),
            ("Game time 0:02.000", at(2))
        );
        // The flash is for the first game time only.
        assert_eq!(latest.arrived, first);
        let previous: Vec<_> = actions.previous().map(|a| a.word.as_str()).collect();
        assert_eq!(previous, ["Start"]);

        // Another action ends the run.
        actions.apply(&action(TimerAction::Split, 1), at(3), first);
        actions.apply(&game_time(3), at(4), first);
        let words: Vec<_> = std::iter::once(actions.latest().unwrap())
            .chain(actions.previous())
            .map(|a| a.word.as_str())
            .collect();
        assert_eq!(words, ["Game time 0:03.000", "Split", "Game time 0:02.000"]);
    }

    #[test]
    fn a_dropped_action_is_shown_as_not_sent() {
        let mut actions = LastActions::default();
        actions.apply(&action(TimerAction::Split, 0), at(0), Instant::now());
        assert!(!actions.latest().unwrap().sent);
        actions.apply(&action(TimerAction::Split, 2), at(1), Instant::now());
        assert!(actions.latest().unwrap().sent);
    }

    #[test]
    fn custom_variables_and_other_events_are_not_shown() {
        let mut actions = LastActions::default();
        let variable = TimerAction::SetVariable {
            key: "Area".to_owned(),
            value: "Los Santos".to_owned(),
        };
        actions.apply(&action(variable, 1), at(0), Instant::now());
        actions.apply(&RunnerEvent::GameDetached, at(0), Instant::now());
        assert_eq!(actions.latest(), None);
    }

    #[test]
    fn each_action_has_its_word() {
        let cases = [
            (TimerAction::Start, "Start"),
            (TimerAction::Split, "Split"),
            (TimerAction::SkipSplit, "Skip split"),
            (TimerAction::UndoSplit, "Undo split"),
            (TimerAction::Reset, "Reset"),
            (TimerAction::PauseGameTime, "Pause game time"),
            (TimerAction::ResumeGameTime, "Resume game time"),
            (
                TimerAction::SetGameTime(time::Duration::new(5025, 600_000_000)),
                "Game time 1:23:45.600",
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(word(&action).as_deref(), Some(expected), "{action:?}");
        }
    }

    #[test]
    fn game_times_under_an_hour_leave_out_the_hours() {
        assert_eq!(game_time(time::Duration::new(187, 250_000_000)), "3:07.250");
        assert_eq!(game_time(time::Duration::ZERO), "0:00.000");
        assert_eq!(
            game_time(time::Duration::new(-1, -500_000_000)),
            "-0:01.500"
        );
        assert_eq!(
            game_time(time::Duration::new(36_000, 999_999_999)),
            "10:00:00.999"
        );
    }
}
