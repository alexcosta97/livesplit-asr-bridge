//! A timer's tracked state (spec §5.2): a copy of its phase, current split
//! index and which segments were split or skipped, kept up to date from the
//! timer's answers and events so the auto splitter's questions are answered
//! immediately.
//!
//! The copy starts from `getCurrentState`, follows every event, and is
//! corrected by asking again every 2 seconds. `getCurrentState` only gives
//! the phase and the index, so the timer is also asked which of the earlier
//! segments have a split time (`getCurrentRunSplitTime`), and the current
//! segment's name (`getSegmentName`), which the Last action card shows on a
//! split.

use livesplit_auto_splitting::TimerState;

/// The phase of a timer's attempt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    NotRunning,
    Running,
    Paused,
    Ended,
}

impl From<Phase> for TimerState {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::NotRunning => TimerState::NotRunning,
            Phase::Running => TimerState::Running,
            Phase::Paused => TimerState::Paused,
            Phase::Ended => TimerState::Ended,
        }
    }
}

/// What the Connection tab shows about a timer, like "Running · split 12".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub phase: Phase,
    /// The current split index, counted from 0. `None` when not running, and
    /// when the attempt ended before the timer was tracked, since
    /// `getCurrentState` gives no index once the attempt has ended.
    pub index: Option<usize>,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phase = match self.phase {
            Phase::NotRunning => return f.write_str("Not running"),
            Phase::Running => "Running",
            Phase::Paused => "Paused",
            Phase::Ended => return f.write_str("Ended"),
        };
        match self.index {
            // People count splits from 1.
            Some(index) => write!(f, "{phase} · split {}", index + 1),
            None => f.write_str(phase),
        }
    }
}

/// What is known about an earlier segment of the attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Record {
    /// Not known yet: the timer hasn't been asked.
    Unknown,
    /// The timer has been asked, and hasn't answered yet.
    Asked,
    Split,
    Skipped,
}

/// A question to ask the timer, to fill in the tracked state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    /// `getCurrentState`.
    State,
    /// `getCurrentRunSplitTime` of a segment, in real time: a segment has a
    /// split time if it was split, and none if it was skipped.
    SplitTime { index: usize, attempt: u64 },
    /// `getSegmentName` of a segment.
    SegmentName { index: usize, attempt: u64 },
}

/// A timer's tracked state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tracked {
    phase: Phase,
    index: Option<usize>,
    /// What is known about each segment before `index`.
    segments: Vec<Record>,
    /// The current segment's name, with its index, once the timer gave it.
    name: Option<(usize, String)>,
    /// Which segment's name has been asked for.
    name_asked: Option<usize>,
    /// Changes whenever the segments before `index` may no longer be what
    /// was asked about: on a start, a reset, an undo, or when an answer
    /// shows that events were missed. Answers to questions asked in an
    /// earlier attempt are ignored.
    attempt: u64,
    /// Whether the state has been received at least once.
    known: bool,
}

impl Tracked {
    /// The phase and index, once the timer has told them.
    pub fn summary(&self) -> Option<Summary> {
        self.known.then_some(Summary {
            phase: self.phase,
            index: self.index,
        })
    }

    pub fn state(&self) -> TimerState {
        self.phase.into()
    }

    pub fn current_split_index(&self) -> Option<usize> {
        self.index
    }

    /// Whether the segment at `index` was split (`true`) or skipped
    /// (`false`). `None` for the current segment and later ones, as in
    /// livesplit-core, and for an earlier one the timer hasn't told about
    /// yet.
    pub fn segment_splitted(&self, index: usize) -> Option<bool> {
        match self.segments.get(index)? {
            Record::Split => Some(true),
            Record::Skipped => Some(false),
            Record::Unknown | Record::Asked => None,
        }
    }

    /// The current segment's name, if the timer gave it.
    pub fn segment_name(&self) -> Option<&str> {
        match (&self.name, self.index) {
            (Some((named, name)), Some(index)) if *named == index => Some(name),
            _ => None,
        }
    }

    /// Updates the copy with an event from the timer, like `Splitted`.
    pub fn event(&mut self, event: &str) {
        if !self.known {
            // Wait for the state, which the timer is asked for first.
            return;
        }
        match event {
            "Started" => self.start(),
            "Splitted" => self.advance(Record::Split, self.phase),
            "Finished" => self.advance(Record::Split, Phase::Ended),
            "SplitSkipped" => self.advance(Record::Skipped, self.phase),
            "SplitUndone" => {
                if let Some(index) = self.index.and_then(|index| index.checked_sub(1)) {
                    self.index = Some(index);
                    self.segments.truncate(index);
                    if self.phase == Phase::Ended {
                        self.phase = Phase::Running;
                    }
                    self.attempt += 1;
                }
            }
            "Reset" => self.reset(),
            "Paused" if self.phase == Phase::Running => self.phase = Phase::Paused,
            "Resumed" | "PausesUndoneAndResumed" if self.phase == Phase::Paused => {
                self.phase = Phase::Running;
            }
            _ => {}
        }
    }

    /// Updates the copy with the timer's answer to `getCurrentState`. When
    /// the answer doesn't match the copy, events were missed, so what is
    /// known about the earlier segments is asked again.
    pub fn state_answer(&mut self, phase: Phase, index: Option<usize>) {
        let first = !self.known;
        self.known = true;
        match phase {
            Phase::NotRunning => {
                if first || self.phase != Phase::NotRunning {
                    self.reset();
                }
            }
            Phase::Running | Phase::Paused => {
                let index = index.unwrap_or_default();
                if first || self.index != Some(index) || self.phase == Phase::NotRunning {
                    self.phase = phase;
                    self.index = Some(index);
                    self.segments = vec![Record::Unknown; index];
                    self.attempt += 1;
                } else {
                    self.phase = phase;
                }
            }
            Phase::Ended => {
                if self.phase != Phase::Ended {
                    // The copy missed the end of the attempt. The index is
                    // the number of segments, which the answer doesn't give.
                    self.phase = Phase::Ended;
                    self.index = None;
                    self.segments.clear();
                    self.attempt += 1;
                }
            }
        }
    }

    /// Updates the copy with the timer's answer about a segment's split
    /// time: `true` if it has one.
    pub fn split_time_answer(&mut self, index: usize, attempt: u64, has_time: bool) {
        if attempt != self.attempt {
            return;
        }
        if let Some(record) = self.segments.get_mut(index) {
            *record = if has_time {
                Record::Split
            } else {
                Record::Skipped
            };
        }
    }

    /// Updates the copy with a segment's name.
    pub fn segment_name_answer(&mut self, index: usize, attempt: u64, name: String) {
        if attempt == self.attempt {
            self.name = Some((index, name));
        }
    }

    /// What to ask the timer to fill in the copy, marking it asked.
    pub fn queries(&mut self) -> Vec<Query> {
        let attempt = self.attempt;
        let mut queries = Vec::new();
        for (index, record) in self.segments.iter_mut().enumerate() {
            if *record == Record::Unknown {
                *record = Record::Asked;
                queries.push(Query::SplitTime { index, attempt });
            }
        }
        if matches!(self.phase, Phase::Running | Phase::Paused)
            && let Some(index) = self.index
            && self.name_asked != Some(index)
        {
            self.name_asked = Some(index);
            queries.push(Query::SegmentName { index, attempt });
        }
        queries
    }

    fn start(&mut self) {
        self.phase = Phase::Running;
        self.index = Some(0);
        self.segments.clear();
        self.name = None;
        self.name_asked = None;
        self.attempt += 1;
    }

    fn reset(&mut self) {
        self.phase = Phase::NotRunning;
        self.index = None;
        self.segments.clear();
        self.name = None;
        self.name_asked = None;
        self.attempt += 1;
    }

    /// Moves to the next segment, recording the current one.
    fn advance(&mut self, record: Record, phase: Phase) {
        let Some(index) = self.index else {
            return;
        };
        self.segments.truncate(index);
        self.segments.resize(index, Record::Unknown);
        self.segments.push(record);
        self.index = Some(index + 1);
        self.phase = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(index: usize) -> Tracked {
        let mut tracked = Tracked::default();
        tracked.state_answer(Phase::Running, Some(index));
        tracked
    }

    fn summary(tracked: &Tracked) -> String {
        tracked.summary().map(|s| s.to_string()).unwrap_or_default()
    }

    #[test]
    fn nothing_is_known_before_the_first_answer() {
        let mut tracked = Tracked::default();
        tracked.event("Started");
        assert_eq!(tracked.summary(), None);
        assert_eq!(tracked.state(), TimerState::NotRunning);
        assert_eq!(tracked.current_split_index(), None);
    }

    #[test]
    fn connecting_mid_run_starts_from_the_timer_state() {
        let mut tracked = running(12);
        assert_eq!(tracked.state(), TimerState::Running);
        assert_eq!(tracked.current_split_index(), Some(12));
        assert_eq!(summary(&tracked), "Running · split 13");
        // The earlier segments are asked about, and the current one's name.
        let queries = tracked.queries();
        assert_eq!(queries.len(), 13);
        assert_eq!(tracked.segment_splitted(3), None);
        let attempt = tracked.attempt;
        tracked.split_time_answer(3, attempt, true);
        tracked.split_time_answer(4, attempt, false);
        assert_eq!(tracked.segment_splitted(3), Some(true));
        assert_eq!(tracked.segment_splitted(4), Some(false));
        // Nothing is asked twice.
        assert!(tracked.queries().is_empty());
    }

    #[test]
    fn events_update_the_state() {
        let mut tracked = Tracked::default();
        tracked.state_answer(Phase::NotRunning, None);
        assert_eq!(summary(&tracked), "Not running");

        tracked.event("Started");
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::Running, Some(0))
        );
        tracked.event("Splitted");
        tracked.event("SplitSkipped");
        assert_eq!(tracked.current_split_index(), Some(2));
        assert_eq!(tracked.segment_splitted(0), Some(true));
        assert_eq!(tracked.segment_splitted(1), Some(false));
        assert_eq!(tracked.segment_splitted(2), None);

        tracked.event("Paused");
        assert_eq!(tracked.state(), TimerState::Paused);
        tracked.event("Resumed");
        assert_eq!(tracked.state(), TimerState::Running);

        tracked.event("SplitUndone");
        assert_eq!(tracked.current_split_index(), Some(1));
        assert_eq!(tracked.segment_splitted(1), None);

        tracked.event("Splitted");
        tracked.event("Finished");
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::Ended, Some(3))
        );
        assert_eq!(summary(&tracked), "Ended");
        tracked.event("SplitUndone");
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::Running, Some(2))
        );

        tracked.event("Reset");
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::NotRunning, None)
        );
        assert_eq!(tracked.segment_splitted(0), None);
        // Events that don't change the state are ignored.
        tracked.event("GameTimeSet");
        tracked.event("Paused");
        assert_eq!(tracked.state(), TimerState::NotRunning);
    }

    #[test]
    fn a_matching_answer_keeps_what_is_known() {
        let mut tracked = Tracked::default();
        tracked.state_answer(Phase::NotRunning, None);
        tracked.event("Started");
        tracked.event("Splitted");
        tracked.state_answer(Phase::Running, Some(1));
        assert_eq!(tracked.segment_splitted(0), Some(true));
        assert!(
            tracked
                .queries()
                .iter()
                .all(|q| matches!(q, Query::SegmentName { .. }))
        );
        tracked.state_answer(Phase::Paused, Some(1));
        assert_eq!(tracked.state(), TimerState::Paused);
        assert_eq!(tracked.segment_splitted(0), Some(true));
    }

    #[test]
    fn changes_made_on_the_timer_are_picked_up_by_the_next_answer() {
        let mut tracked = Tracked::default();
        tracked.state_answer(Phase::NotRunning, None);
        tracked.event("Started");
        tracked.event("Splitted");
        // Missed: the runner split twice more on the timer.
        tracked.state_answer(Phase::Running, Some(3));
        assert_eq!(tracked.current_split_index(), Some(3));
        assert_eq!(tracked.segment_splitted(0), None);
        let queries = tracked.queries();
        let split_times = queries
            .iter()
            .filter(|q| matches!(q, Query::SplitTime { .. }))
            .count();
        assert_eq!(split_times, 3);

        // Missed: a reset.
        tracked.state_answer(Phase::NotRunning, None);
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::NotRunning, None)
        );

        // Missed: the end of an attempt, whose length isn't known.
        tracked.state_answer(Phase::Running, Some(0));
        tracked.state_answer(Phase::Ended, None);
        assert_eq!(
            (tracked.state(), tracked.current_split_index()),
            (TimerState::Ended, None)
        );
    }

    #[test]
    fn answers_from_an_earlier_attempt_are_ignored() {
        let mut tracked = running(2);
        let old = tracked.attempt;
        tracked.queries();
        tracked.event("Reset");
        tracked.event("Started");
        tracked.event("Splitted");
        tracked.event("Splitted");
        tracked.split_time_answer(0, old, false);
        tracked.segment_name_answer(2, old, "Old".to_owned());
        assert_eq!(tracked.segment_splitted(0), Some(true));
        assert_eq!(tracked.segment_name(), None);
    }

    #[test]
    fn the_current_segment_name_is_asked_once_per_segment() {
        let mut tracked = Tracked::default();
        tracked.state_answer(Phase::NotRunning, None);
        assert!(tracked.queries().is_empty());
        tracked.event("Started");
        let attempt = tracked.attempt;
        assert_eq!(
            tracked.queries(),
            [Query::SegmentName { index: 0, attempt }]
        );
        assert!(tracked.queries().is_empty());
        tracked.segment_name_answer(0, attempt, "Gym Moves".to_owned());
        assert_eq!(tracked.segment_name(), Some("Gym Moves"));
        tracked.event("Splitted");
        // The name was for the previous segment.
        assert_eq!(tracked.segment_name(), None);
        assert_eq!(
            tracked.queries(),
            [Query::SegmentName { index: 1, attempt }]
        );
    }
}
