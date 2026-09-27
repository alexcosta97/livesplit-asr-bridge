//! LiveSplit One's server protocol (`livesplit-core`'s
//! `networking::server_protocol`): the commands sent for the auto splitter's
//! actions, and the responses LiveSplit One sends back.
//!
//! Each command is a JSON object named by its `command` field, in camelCase.
//! LiveSplit One answers every command, in order, with `{"success": ...}` or
//! `{"error": {"code": ..., "message": ...}}`, and also sends events such as
//! `{"event": "Splitted"}`, which aren't answers.
//!
//! The Server also asks questions of its own to track the timer's state
//! (spec §5.2): `getCurrentState`, `getCurrentRunSplitTime` and
//! `getSegmentName`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::tracked::{Phase, Query};
use crate::runner::TimerAction;

/// The name of the command sent for `action`.
pub fn command_name(action: &TimerAction) -> &'static str {
    match action {
        TimerAction::Start => "start",
        TimerAction::Split => "split",
        TimerAction::SkipSplit => "skipSplit",
        TimerAction::UndoSplit => "undoSplit",
        TimerAction::Reset => "reset",
        TimerAction::SetGameTime(_) => "setGameTime",
        TimerAction::PauseGameTime => "pauseGameTime",
        TimerAction::ResumeGameTime => "resumeGameTime",
        TimerAction::SetVariable { .. } => "setCustomVariable",
    }
}

/// The command sent for `action`, as JSON text.
///
/// `setGameTime` needs no `initializeGameTime` first: setting the game time
/// initialises it, as it does when LiveSplit One runs an auto splitter
/// itself. `reset` leaves out `saveAttempt`, so LiveSplit One decides, again
/// as it does itself.
pub fn encode(action: &TimerAction) -> String {
    let command = command_name(action);
    let value = match action {
        TimerAction::SetGameTime(time) => json!({ "command": command, "time": time_text(*time) }),
        TimerAction::SetVariable { key, value } => {
            json!({ "command": command, "key": key, "value": value })
        }
        _ => json!({ "command": command }),
    };
    value.to_string()
}

/// The question asked for `query`, as JSON text.
pub fn encode_query(query: &Query) -> String {
    let value = match query {
        Query::State => json!({ "command": "getCurrentState" }),
        Query::SplitTime { index, .. } => json!({
            "command": "getCurrentRunSplitTime",
            "index": index,
            "timingMethod": "RealTime",
        }),
        Query::SegmentName { index, .. } => {
            json!({ "command": "getSegmentName", "index": index })
        }
    };
    value.to_string()
}

/// Reads the answer to `getCurrentState`, like `{"state": "Running",
/// "index": 12}`.
pub fn parse_state(value: &Value) -> Option<(Phase, Option<usize>)> {
    let phase = match value.get("state")?.as_str()? {
        "NotRunning" => Phase::NotRunning,
        "Running" => Phase::Running,
        "Paused" => Phase::Paused,
        "Ended" => Phase::Ended,
        _ => return None,
    };
    let index = value
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|index| usize::try_from(index).ok());
    Some((phase, index))
}

/// A time as LiveSplit One parses it: seconds, with nine decimal places.
fn time_text(time: livesplit_auto_splitting::time::Duration) -> String {
    let sign = if time.is_negative() { "-" } else { "" };
    format!(
        "{sign}{}.{:09}",
        time.whole_seconds().unsigned_abs(),
        time.subsec_nanoseconds().unsigned_abs()
    )
}

/// A message from LiveSplit One, as far as sending commands is concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Received {
    /// A command succeeded. Questions carry their answer; other commands
    /// carry `null`.
    Success(Value),
    /// A command was rejected, for example a split with no run in progress.
    Error {
        code: String,
        message: Option<String>,
    },
    /// An event, like `Splitted`. Events are sent alongside the answer to a
    /// command, not instead of it.
    Event(String),
    /// Anything else.
    Other,
}

#[derive(Deserialize)]
struct ErrorBody {
    code: String,
    message: Option<String>,
}

/// Reads a text message from LiveSplit One.
pub fn parse(text: &str) -> Received {
    let Ok(Value::Object(mut object)) = serde_json::from_str::<Value>(text) else {
        return Received::Other;
    };
    if let Some(value) = object.remove("success") {
        return Received::Success(value);
    }
    if let Some(Value::String(event)) = object.remove("event") {
        return Received::Event(event);
    }
    match object
        .remove("error")
        .map(serde_json::from_value::<ErrorBody>)
    {
        Some(Ok(ErrorBody { code, message })) => Received::Error { code, message },
        Some(Err(_)) => Received::Error {
            code: "Unknown".to_owned(),
            message: None,
        },
        None => Received::Other,
    }
}

#[cfg(test)]
mod tests {
    use livesplit_auto_splitting::time::Duration;

    use super::*;

    #[test]
    fn each_action_produces_its_command() {
        let cases = [
            (TimerAction::Start, r#"{"command":"start"}"#),
            (TimerAction::Split, r#"{"command":"split"}"#),
            (TimerAction::SkipSplit, r#"{"command":"skipSplit"}"#),
            (TimerAction::UndoSplit, r#"{"command":"undoSplit"}"#),
            (TimerAction::Reset, r#"{"command":"reset"}"#),
            (TimerAction::PauseGameTime, r#"{"command":"pauseGameTime"}"#),
            (
                TimerAction::ResumeGameTime,
                r#"{"command":"resumeGameTime"}"#,
            ),
            (
                TimerAction::SetGameTime(Duration::new(5025, 600_000_000)),
                r#"{"command":"setGameTime","time":"5025.600000000"}"#,
            ),
            (
                TimerAction::SetVariable {
                    key: "Area".to_owned(),
                    value: "Los \"Santos\"".to_owned(),
                },
                r#"{"command":"setCustomVariable","key":"Area","value":"Los \"Santos\""}"#,
            ),
        ];
        for (action, expected) in cases {
            let encoded = encode(&action);
            // Compared as JSON, so the order of the fields doesn't matter.
            assert_eq!(
                serde_json::from_str::<Value>(&encoded).unwrap(),
                serde_json::from_str::<Value>(expected).unwrap(),
                "{action:?}: {encoded}"
            );
        }
    }

    #[test]
    fn game_times_keep_their_sign_and_precision() {
        assert_eq!(time_text(Duration::ZERO), "0.000000000");
        assert_eq!(time_text(Duration::new(1, 5)), "1.000000005");
        assert_eq!(time_text(Duration::new(0, -500_000_000)), "-0.500000000");
        assert_eq!(time_text(Duration::new(-90, -250_000_000)), "-90.250000000");
    }

    #[test]
    fn reads_answers_and_events() {
        assert_eq!(parse(r#"{"success":null}"#), Received::Success(Value::Null));
        assert_eq!(
            parse(r#"{"success":"1:23.45"}"#),
            Received::Success(json!("1:23.45"))
        );
        assert_eq!(
            parse(r#"{"error":{"code":"NoRunInProgress"}}"#),
            Received::Error {
                code: "NoRunInProgress".to_owned(),
                message: None,
            }
        );
        assert_eq!(
            parse(r#"{"error":{"code":"InvalidCommand","message":"unknown variant"}}"#),
            Received::Error {
                code: "InvalidCommand".to_owned(),
                message: Some("unknown variant".to_owned()),
            }
        );
        assert_eq!(
            parse(r#"{"event":"Splitted"}"#),
            Received::Event("Splitted".to_owned())
        );
        assert_eq!(parse("not json"), Received::Other);
        assert_eq!(parse("[1, 2]"), Received::Other);
    }

    #[test]
    fn each_query_produces_its_command() {
        let cases = [
            (Query::State, r#"{"command":"getCurrentState"}"#),
            (
                Query::SplitTime {
                    index: 3,
                    attempt: 1,
                },
                r#"{"command":"getCurrentRunSplitTime","index":3,"timingMethod":"RealTime"}"#,
            ),
            (
                Query::SegmentName {
                    index: 3,
                    attempt: 1,
                },
                r#"{"command":"getSegmentName","index":3}"#,
            ),
        ];
        for (query, expected) in cases {
            assert_eq!(
                serde_json::from_str::<Value>(&encode_query(&query)).unwrap(),
                serde_json::from_str::<Value>(expected).unwrap(),
                "{query:?}"
            );
        }
    }

    #[test]
    fn reads_the_current_state() {
        let state = |text: &str| parse_state(&serde_json::from_str(text).unwrap());
        assert_eq!(
            state(r#"{"state":"NotRunning"}"#),
            Some((Phase::NotRunning, None))
        );
        assert_eq!(
            state(r#"{"state":"Running","index":12}"#),
            Some((Phase::Running, Some(12)))
        );
        assert_eq!(
            state(r#"{"state":"Paused","index":0}"#),
            Some((Phase::Paused, Some(0)))
        );
        assert_eq!(state(r#"{"state":"Ended"}"#), Some((Phase::Ended, None)));
        assert_eq!(state(r#"{"state":"Sleeping"}"#), None);
        assert_eq!(state("null"), None);
    }
}
