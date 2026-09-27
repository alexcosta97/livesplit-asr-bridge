//! End to end (spec §12): the test auto splitter that starts, splits and
//! logs on a schedule runs in the Runner while a fake LiveSplit One client
//! is connected to the Server.

use std::{
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use livesplit_auto_splitting::settings::Map;
use tokio::{net::TcpStream, time::timeout};
use tokio_tungstenite::{client_async, tungstenite::Message};

use crate::{
    runner::{Runner, RunnerEvent, TimerAction},
    server::{Server, ServerEvent},
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
fn a_connected_timer_receives_the_auto_splitters_commands_in_order() {
    let (server, server_events) = Server::new(0, || {});
    let ServerEvent::Listening { port } = wait_for(&server_events, |event| {
        matches!(event, ServerEvent::Listening { .. })
    }) else {
        unreachable!()
    };
    let (runner, events) = Runner::new(server.link(), || {});

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    let mut client = runtime.block_on(async {
        let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        let (client, _) = client_async(format!("ws://127.0.0.1:{port}/"), stream)
            .await
            .unwrap();
        client
    });
    wait_for(&server_events, |event| {
        matches!(event, ServerEvent::TimerConnected { .. })
    });

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schedule.wasm");
    std::fs::write(&path, wat::parse_str(SCHEDULE).unwrap()).unwrap();
    runner.load(path, Map::new());

    // The fake LiveSplit One answers every command, as the real one does.
    let received = runtime.block_on(async {
        let mut received = Vec::new();
        while received.last().map(String::as_str) != Some(r#"{"command":"reset"}"#) {
            let message = timeout(TIMEOUT, client.next())
                .await
                .expect("no command arrived")
                .expect("the connection closed")
                .unwrap();
            if let Message::Text(text) = message {
                received.push(text.to_string());
                let answer = Message::text(r#"{"success":null}"#);
                client.send(answer).await.unwrap();
            }
        }
        received
    });
    assert_eq!(
        received,
        [
            r#"{"command":"start"}"#,
            r#"{"command":"split"}"#,
            r#"{"command":"setGameTime","time":"1.500000000"}"#,
            r#"{"command":"split"}"#,
            r#"{"command":"reset"}"#,
        ]
    );

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
        }
    );
    // Answers aren't rejections.
    assert!(
        server_events
            .try_iter()
            .all(|event| !matches!(event, ServerEvent::CommandRejected { .. }))
    );
}
