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

use super::*;

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

    /// Reads the next message the client receives.
    fn receive(&self, client: &mut Client) -> Message {
        self.runtime.block_on(async {
            timeout(TIMEOUT, client.next())
                .await
                .expect("no message arrived")
                .expect("the connection closed")
                .expect("the connection failed")
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

#[test]
fn sends_messages_to_every_timer_from_any_thread() {
    let (harness, port) = Harness::listening();
    let (mut first, _, _) = harness.connect(port);
    let (mut second, _, _) = harness.connect(port);

    assert_eq!(harness.server().send_to_all("hello"), 2);
    assert_eq!(harness.receive(&mut first), Message::text("hello"));
    assert_eq!(harness.receive(&mut second), Message::text("hello"));
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
