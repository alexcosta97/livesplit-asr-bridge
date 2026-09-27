//! End to end (spec §12): the test auto splitter that starts, splits and
//! logs on a schedule runs in the Runner while a fake LiveSplit One client
//! is connected to the Server. The client answers and sends events as
//! LiveSplit One does, and its events update the tracked state.

use std::{
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use livesplit_auto_splitting::{TimerState, settings::Map};

use crate::{
    runner::{Runner, RunnerEvent, TimerAction, TimerLink},
    server::{
        Server, ServerEvent,
        fake_timer::{FakeTimer, Model, eventually},
    },
};

/// Long enough for a slow CI machine to compile the auto splitter.
const TIMEOUT: Duration = Duration::from_secs(30);

const SCHEDULE: &str = include_str!("runner/test_auto_splitters/schedule.wat");

/// Waits for a matching event, failing the test on timeout.
fn wait_for<T: std::fmt::Debug>(events: &Receiver<T>, matches: impl Fn(&T) -> bool) -> T {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = events.recv_timeout(left).expect("timed out");
        if matches(&event) {
            return event;
        }
    }
}

#[test]
fn a_connected_timer_receives_the_commands_in_order_and_its_events_update_the_state() {
    let (server, server_events) = Server::new(0, || {});
    let ServerEvent::Listening { port } = wait_for(&server_events, |event| {
        matches!(event, ServerEvent::Listening { .. })
    }) else {
        unreachable!()
    };
    let link = server.link();
    let (runner, events) = Runner::new(link.clone(), || {});

    let names = ["Gym Moves", "Ryder", "Sweet", "Big Smoke"];
    let timer = FakeTimer::connect(port, Model::new(&names));
    wait_for(&server_events, |event| {
        matches!(event, ServerEvent::TimerConnected { .. })
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schedule.wasm");
    std::fs::write(&path, wat::parse_str(SCHEDULE).unwrap()).unwrap();
    runner.load(path, Map::new());

    // Each action is reported as sent to the one timer.
    let reset = wait_for(&events, |event| {
        matches!(
            event,
            RunnerEvent::TimerAction {
                action: TimerAction::Reset,
                ..
            }
        )
    });
    assert_eq!(
        reset,
        RunnerEvent::TimerAction {
            action: TimerAction::Reset,
            sent_to: 1,
            segment: None,
        }
    );

    // The fake LiveSplit One received the commands in order.
    eventually("the reset", || timer.actions().len() == 5);
    assert_eq!(
        timer.actions(),
        [
            r#"{"command":"start"}"#,
            r#"{"command":"split"}"#,
            r#"{"command":"setGameTime","time":"1.500000000"}"#,
            r#"{"command":"split"}"#,
            r#"{"command":"reset"}"#,
        ]
    );

    // Its events updated the tracked state, step by step.
    let mut states = Vec::new();
    while states.last().map(String::as_str) != Some("Not running") || states.len() < 2 {
        if let ServerEvent::TimerState { summary, .. } = wait_for(&server_events, |event| {
            matches!(
                event,
                ServerEvent::TimerState { .. } | ServerEvent::CommandRejected { .. }
            )
        }) {
            states.push(summary.to_string());
        } else {
            panic!("a command was rejected; states so far: {states:?}");
        }
    }
    assert_eq!(
        states,
        [
            "Not running",
            "Running · split 1",
            "Running · split 2",
            "Running · split 3",
            "Not running",
        ]
    );
    assert_eq!(link.state(), TimerState::NotRunning);
}
