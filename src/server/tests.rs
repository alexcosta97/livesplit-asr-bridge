//! Runs the Server on a free port and connects to it with real WebSocket
//! clients over loopback.

use std::{
    io::Write,
    net::SocketAddr,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

use futures_util::StreamExt;
use tokio::{net::TcpStream, runtime::Runtime, time::timeout};
use tokio_tungstenite::{WebSocketStream, client_async, tungstenite::Message};

use super::{
    fake_timer::{FakeTimer, Model, eventually},
    *,
};

/// Long enough for a slow CI machine; only reached when a test fails.
const TIMEOUT: Duration = Duration::from_secs(10);

type Client = WebSocketStream<TcpStream>;

struct Harness {
    server: Option<Server>,
    events: Receiver<ServerEvent>,
    /// Runs the test's clients.
    runtime: Runtime,
}

impl Harness {
    /// Starts a Server on `port` (0 for a free one).
    fn new(port: u16) -> Self {
        let (server, events) = Server::new(port, || {});
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();
        Self {
            server: Some(server),
            events,
            runtime,
        }
    }

    /// Starts a Server on a free port and returns the port it listens on.
    fn listening() -> (Self, u16) {
        let harness = Self::new(0);
        let port = harness.wait_for_listening();
        assert_ne!(port, 0);
        (harness, port)
    }

    fn server(&self) -> &Server {
        self.server.as_ref().unwrap()
    }

    /// Collects events until one matches, failing the test on timeout.
    fn wait_for(&self, matches: impl Fn(&ServerEvent) -> bool) -> Vec<ServerEvent> {
        let deadline = Instant::now() + TIMEOUT;
        let mut seen = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.events.recv_timeout(left) {
                Ok(event) => {
                    let done = matches(&event);
                    seen.push(event);
                    if done {
                        return seen;
                    }
                }
                Err(RecvTimeoutError::Timeout) => panic!("timed out; events so far: {seen:?}"),
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("the server stopped; events so far: {seen:?}")
                }
            }
        }
    }

    fn wait_for_listening(&self) -> u16 {
        let seen = self.wait_for(|event| matches!(event, ServerEvent::Listening { .. }));
        match seen.last() {
            Some(ServerEvent::Listening { port }) => *port,
            _ => unreachable!(),
        }
    }

    /// Connects a WebSocket client, and waits for the server to report it.
    /// Returns the client, its id and its address.
    fn connect(&self, port: u16) -> (Client, ConnectionId, SocketAddr) {
        let client = self.runtime.block_on(async {
            let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let url = format!("ws://127.0.0.1:{port}/");
            let (client, _) = timeout(TIMEOUT, client_async(url, stream))
                .await
                .expect("the handshake timed out")
                .expect("the handshake failed");
            client
        });
        let address = client.get_ref().local_addr().unwrap();
        let seen = self.wait_for(|event| {
            matches!(event, ServerEvent::TimerConnected { address: a, .. } if *a == address)
        });
        let Some(ServerEvent::TimerConnected { id, .. }) = seen.last() else {
            unreachable!()
        };
        (client, *id, address)
    }

    /// Connects a fake LiveSplit One with the timer in `model`'s state, and
    /// waits for the server to report it.
    fn fake(&self, port: u16, model: Model) -> (FakeTimer, ConnectionId) {
        let timer = FakeTimer::connect(port, model);
        let seen = self.wait_for(|event| matches!(event, ServerEvent::TimerConnected { .. }));
        let Some(ServerEvent::TimerConnected { id, .. }) = seen.last() else {
            unreachable!()
        };
        (timer, *id)
    }

    /// Waits for the client's connection to end, returning whether the
    /// server sent a Close frame first.
    fn closed_by_server(&self, client: &mut Client) -> bool {
        self.runtime.block_on(async {
            let mut got_close = false;
            loop {
                match timeout(TIMEOUT, client.next())
                    .await
                    .expect("the connection stayed open")
                {
                    Some(Ok(Message::Close(_))) => got_close = true,
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => return got_close,
                }
            }
        })
    }
}

fn disconnected(id: ConnectionId) -> impl Fn(&ServerEvent) -> bool {
    move |event| matches!(event, ServerEvent::TimerDisconnected { id: i, .. } if *i == id)
}

#[test]
fn listens_on_a_free_port() {
    let (_harness, port) = Harness::listening();
    assert_ne!(port, 0);
}

#[test]
fn reports_a_timer_connecting_and_disconnecting() {
    let (harness, port) = Harness::listening();
    let (mut client, id, address) = harness.connect(port);
    assert_eq!(address.ip(), std::net::Ipv4Addr::LOCALHOST);

    harness.runtime.block_on(client.close(None)).unwrap();
    let seen = harness.wait_for(disconnected(id));
    assert_eq!(
        seen.last(),
        Some(&ServerEvent::TimerDisconnected { id, address })
    );
}

#[test]
fn reports_each_timer() {
    let (harness, port) = Harness::listening();
    let (_first, first_id, first_address) = harness.connect(port);
    let (_second, second_id, second_address) = harness.connect(port);
    assert_ne!(first_id, second_id);
    assert_ne!(first_address, second_address);
}

#[test]
fn a_dropped_connection_is_reported() {
    let (harness, port) = Harness::listening();
    let (client, id, _) = harness.connect(port);
    // No Close frame: the TCP connection just ends.
    drop(client);
    harness.wait_for(disconnected(id));
}

const SPLIT: &str = r#"{"command":"split"}"#;

#[test]
fn sends_commands_to_every_timer_from_any_thread() {
    let (harness, port) = Harness::listening();
    let (first, _) = harness.fake(port, Model::new(&["A"]));
    let (second, _) = harness.fake(port, Model::new(&["A"]));

    let link = harness.server().link();
    let sent = std::thread::spawn(move || link.send(TimerAction::Split))
        .join()
        .unwrap();
    assert_eq!(sent, 2);
    eventually("the split", || {
        first.actions() == [SPLIT] && second.actions() == [SPLIT]
    });
}

#[test]
fn with_no_timer_commands_are_dropped_not_queued() {
    let (harness, port) = Harness::listening();
    let link = harness.server().link();
    assert_eq!(link.send(TimerAction::Start), 0);

    // A timer connecting afterwards gets only what is sent from then on.
    let (timer, _) = harness.fake(port, Model::new(&["A"]));
    assert_eq!(link.send(TimerAction::Split), 1);
    eventually("the split", || !timer.actions().is_empty());
    assert_eq!(timer.actions(), [SPLIT]);
}

#[test]
fn a_rejected_command_is_reported_with_its_name() {
    let (harness, port) = Harness::listening();
    let (timer, id) = harness.fake(port, Model::new(&["A"]));
    let link = harness.server().link();

    // The start succeeds with an event then its answer, the first split
    // finishes the run, and the second is rejected.
    link.send(TimerAction::Start);
    link.send(TimerAction::Split);
    link.send(TimerAction::Split);

    let seen = harness.wait_for(|event| matches!(event, ServerEvent::CommandRejected { .. }));
    let rejections: Vec<_> = seen
        .iter()
        .filter(|event| matches!(event, ServerEvent::CommandRejected { .. }))
        .collect();
    let Some(ServerEvent::CommandRejected {
        id: rejected,
        command,
        code,
        ..
    }) = rejections.first()
    else {
        unreachable!()
    };
    assert_eq!(
        (*rejected, command.as_deref(), code.as_str()),
        (id, Some("split"), "RunFinished")
    );
    assert_eq!(rejections.len(), 1, "{seen:?}");
    assert!(!rejections[0].is_error());
    assert_eq!(timer.actions().len(), 3);
}

/// A timer mid-run with 6 segments: 3 split, the 4th skipped, the 5th split,
/// and running on the 6th.
fn mid_run() -> Model {
    let mut model = Model::new(&["A", "B", "C", "D", "E", "F"]);
    for command in ["start", "split", "split", "split", "skipSplit", "split"] {
        model.act(command).unwrap();
    }
    model
}

#[test]
fn connecting_mid_run_reports_the_state_and_split_index() {
    let (harness, port) = Harness::listening();
    let (_timer, id) = harness.fake(port, mid_run());
    let link = harness.server().link();

    let seen = harness.wait_for(|event| matches!(event, ServerEvent::TimerState { .. }));
    let Some(ServerEvent::TimerState {
        id: tracked,
        summary,
        ..
    }) = seen.last()
    else {
        unreachable!()
    };
    assert_eq!(*tracked, id);
    assert_eq!(summary.to_string(), "Running · split 6");
    assert_eq!(link.state(), TimerState::Running);
    assert_eq!(link.current_split_index(), Some(5));

    // Which earlier segments were split or skipped is asked separately.
    eventually("the segments", || link.segment_splitted(4).is_some());
    let segments: Vec<_> = (0..6).map(|index| link.segment_splitted(index)).collect();
    assert_eq!(
        segments,
        [
            Some(true),
            Some(true),
            Some(true),
            Some(false),
            Some(true),
            None
        ]
    );
    eventually("the segment name", || {
        link.segment_name().as_deref() == Some("F")
    });
}

#[test]
fn events_update_the_tracked_state() {
    let (harness, port) = Harness::listening();
    let (timer, _) = harness.fake(port, Model::new(&["A", "B", "C"]));
    let link = harness.server().link();
    let is = |state: TimerState, index: Option<usize>| {
        let link = link.clone();
        move || link.state() == state && link.current_split_index() == index
    };
    eventually("the first state", || {
        harness.server().shared.connections()[0]
            .tracked
            .summary()
            .is_some()
    });

    link.send(TimerAction::Start);
    eventually("start", is(TimerState::Running, Some(0)));
    link.send(TimerAction::Split);
    eventually("split", is(TimerState::Running, Some(1)));
    assert_eq!(link.segment_splitted(0), Some(true));
    link.send(TimerAction::SkipSplit);
    eventually("skip", is(TimerState::Running, Some(2)));
    assert_eq!(link.segment_splitted(1), Some(false));
    link.send(TimerAction::UndoSplit);
    eventually("undo", is(TimerState::Running, Some(1)));
    assert_eq!(link.segment_splitted(1), None);
    // Pausing and resuming happen on the timer, not from the auto splitter.
    timer.act("pause", true);
    eventually("pause", is(TimerState::Paused, Some(1)));
    timer.act("resume", true);
    eventually("resume", is(TimerState::Running, Some(1)));
    link.send(TimerAction::Split);
    link.send(TimerAction::Split);
    eventually("the end", is(TimerState::Ended, Some(3)));
    link.send(TimerAction::Reset);
    eventually("reset", is(TimerState::NotRunning, None));
    assert_eq!(link.segment_splitted(0), None);
}

#[test]
fn changes_made_on_the_timer_are_picked_up_by_the_next_resync() {
    let (harness, port) = Harness::listening();
    let (timer, _) = harness.fake(port, Model::new(&["A", "B", "C"]));
    let link = harness.server().link();
    eventually("the first state", || {
        harness.server().shared.connections()[0]
            .tracked
            .summary()
            .is_some()
    });

    // The events are lost, as when LiveSplit One doesn't send them.
    let started = Instant::now();
    timer.act("start", false);
    timer.act("split", false);
    eventually("the resync", || link.current_split_index() == Some(1));
    assert_eq!(link.state(), TimerState::Running);
    assert!(
        started.elapsed() < thread::RESYNC_INTERVAL + Duration::from_secs(1),
        "took {:?}",
        started.elapsed()
    );
    eventually("the segment", || link.segment_splitted(0) == Some(true));
}

#[test]
fn the_next_timer_takes_over_when_the_primary_disconnects() {
    let (harness, port) = Harness::listening();
    let (first, first_id) = harness.fake(port, Model::new(&["A"]));
    let (second, _) = harness.fake(port, mid_run());
    let link = harness.server().link();
    eventually("both states", || {
        let connections = harness.server().shared.connections();
        connections.iter().all(|c| c.tracked.summary().is_some())
    });
    // The first to connect is the primary.
    assert_eq!(link.state(), TimerState::NotRunning);

    let asked = |timer: &FakeTimer| {
        timer
            .received()
            .iter()
            .filter(|text| text.contains("getCurrentState"))
            .count()
    };
    let asked_before = asked(&second);
    first.disconnect();
    harness.wait_for(disconnected(first_id));
    assert_eq!(link.state(), TimerState::Running);
    assert_eq!(link.current_split_index(), Some(5));
    // The new primary is asked for its state again.
    eventually("the new primary to be asked", || {
        asked(&second) > asked_before
    });

    second.disconnect();
    eventually("no timer", || {
        harness.server().shared.connections().is_empty()
    });
    assert_eq!(link.state(), TimerState::NotRunning);
    assert_eq!(link.current_split_index(), None);
}

#[test]
fn restart_closes_connections_and_listens_on_the_new_port() {
    let (harness, port) = Harness::listening();
    let (mut first, first_id, _) = harness.connect(port);
    let (mut second, second_id, _) = harness.connect(port);

    harness.server().restart(0);
    let seen = harness.wait_for(|event| matches!(event, ServerEvent::Listening { .. }));
    // Both connections close before the server listens again.
    assert!(seen.iter().any(disconnected(first_id)), "{seen:?}");
    assert!(seen.iter().any(disconnected(second_id)), "{seen:?}");
    assert!(harness.closed_by_server(&mut first));
    assert!(harness.closed_by_server(&mut second));

    let Some(ServerEvent::Listening { port: new_port }) = seen.last() else {
        unreachable!()
    };
    assert_ne!(*new_port, 0);
    harness.connect(*new_port);
}

#[test]
fn restart_on_the_same_port_with_a_timer_connected() {
    let (harness, port) = Harness::listening();
    let (mut client, id, _) = harness.connect(port);

    harness.server().restart(port);
    let seen = harness.wait_for(|event| matches!(event, ServerEvent::Listening { .. }));
    assert!(seen.iter().any(disconnected(id)), "{seen:?}");
    assert_eq!(seen.last(), Some(&ServerEvent::Listening { port }));
    assert!(harness.closed_by_server(&mut client));
    harness.connect(port);
}

#[test]
fn reports_a_port_in_use() {
    let taken = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let port = taken.local_addr().unwrap().port();

    let harness = Harness::new(port);
    let seen = harness.wait_for(|event| matches!(event, ServerEvent::BindFailed { .. }));
    assert_eq!(seen.len(), 1, "{seen:?}");
    assert!(
        matches!(seen[0], ServerEvent::BindFailed { port: p, in_use: true, .. } if p == port),
        "{seen:?}"
    );

    // Restarting on a free port recovers.
    harness.server().restart(0);
    let new_port = harness.wait_for_listening();
    harness.connect(new_port);
}

#[test]
fn something_other_than_a_timer_is_not_counted() {
    let (harness, port) = Harness::listening();
    // Kept open until the server has given up on it.
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.write_all(b"hello there\r\n\r\n").unwrap();
    let address = stream.local_addr().unwrap();
    let seen = harness.wait_for(|event| matches!(event, ServerEvent::HandshakeFailed { .. }));
    assert!(
        matches!(&seen[..], [ServerEvent::HandshakeFailed { address: a, .. }] if *a == address),
        "{seen:?}"
    );

    // A real timer still connects afterwards.
    harness.connect(port);
}

#[test]
fn dropping_the_server_with_a_timer_connected_is_quick() {
    let (mut harness, port) = Harness::listening();
    let (_client, _, _) = harness.connect(port);

    let started = Instant::now();
    drop(harness.server.take());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "took {:?}",
        started.elapsed()
    );
}
