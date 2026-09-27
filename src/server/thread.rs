//! The Server thread: a single-threaded tokio runtime that accepts timers
//! and runs one task per connection.

use std::{
    collections::VecDeque,
    io,
    net::{Ipv4Addr, SocketAddr},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, watch},
    task::JoinSet,
    time::{MissedTickBehavior, interval, sleep, timeout},
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{
        Message,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};

use super::{
    Connection, Outgoing, ServerEvent, Shared,
    protocol::{self, Received},
    tracked::{Query, Tracked},
};

/// How long a new connection has to complete the WebSocket handshake, so a
/// connection that never does doesn't linger.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long closing connections may take, on restart and shutdown, before
/// they are cut off.
const CLOSE_TIMEOUT: Duration = Duration::from_millis(500);
/// How long to wait after a failed accept, for example when the process is
/// out of file descriptors, so the loop doesn't spin.
const ACCEPT_RETRY: Duration = Duration::from_millis(100);
/// How many sent commands are remembered while waiting for their answers, so
/// a timer that never answers can't use unbounded memory.
const MAX_UNANSWERED: usize = 1_000;
/// How often each timer is asked for its state, to pick up changes made on
/// the timer whose events were missed (spec §5.2).
pub(super) const RESYNC_INTERVAL: Duration = Duration::from_secs(2);

/// Instructions for the Server thread.
pub(super) enum Command {
    /// Close every connection and listen on this port instead.
    Restart(u16),
    Shutdown,
}

pub(super) fn run(port: u16, commands: mpsc::UnboundedReceiver<Command>, shared: Arc<Shared>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build();
    match runtime {
        Ok(runtime) => runtime.block_on(serve(port, commands, shared)),
        Err(error) => shared.events.send(ServerEvent::BindFailed {
            port,
            error: format!("couldn't start the network runtime: {error}"),
            in_use: false,
        }),
    }
}

/// Listens on `port` until told to restart or shut down. Each restart closes
/// every connection and listens again, possibly on another port.
async fn serve(mut port: u16, mut commands: mpsc::UnboundedReceiver<Command>, shared: Arc<Shared>) {
    loop {
        let listener = bind(port, &shared).await;
        // Dropping `close` tells every connection to close.
        let (close, closed) = watch::channel(());
        let mut connections = JoinSet::new();

        let command = loop {
            tokio::select! {
                command = commands.recv() => break command,
                accepted = accept(listener.as_ref()) => match accepted {
                    Ok((stream, address)) => {
                        connections.spawn(connection(stream, address, closed.clone(), shared.clone()));
                    }
                    Err(_) => sleep(ACCEPT_RETRY).await,
                },
                // Reaps finished connections, so they don't pile up.
                Some(_) = connections.join_next(), if !connections.is_empty() => {}
            }
        };

        drop(listener);
        drop(close);
        let _ = timeout(CLOSE_TIMEOUT, async {
            while connections.join_next().await.is_some() {}
        })
        .await;
        // Aborting a connection still unregisters it and reports it closed.
        connections.shutdown().await;

        match command {
            Some(Command::Restart(new_port)) => port = new_port,
            Some(Command::Shutdown) | None => return,
        }
    }
}

/// Listens on `port` on all interfaces, reporting the outcome.
async fn bind(port: u16, shared: &Shared) -> Option<TcpListener> {
    // On Unix, tokio sets SO_REUSEADDR, so restarting on the same port works
    // even while closed connections linger in TIME_WAIT.
    match TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await {
        Ok(listener) => {
            let port = listener.local_addr().map_or(port, |address| address.port());
            shared.events.send(ServerEvent::Listening { port });
            Some(listener)
        }
        Err(error) => {
            shared.events.send(ServerEvent::BindFailed {
                port,
                in_use: error.kind() == io::ErrorKind::AddrInUse,
                error: error.to_string(),
            });
            None
        }
    }
}

/// The next connection, or never while not listening.
async fn accept(listener: Option<&TcpListener>) -> io::Result<(TcpStream, SocketAddr)> {
    match listener {
        Some(listener) => listener.accept().await,
        None => std::future::pending().await,
    }
}

/// What woke a connection up.
enum Step {
    Received(Option<Result<Message, tokio_tungstenite::tungstenite::Error>>),
    Outgoing(Outgoing),
    Resync,
    Close,
}

/// What an answer from the timer is for. LiveSplit One answers everything it
/// is sent, in order.
enum Pending {
    /// A command for the auto splitter's action, by name.
    Action(&'static str),
    /// A question the Server asked to track the timer's state.
    Query(Query),
}

/// Runs one connection: the handshake, then messages both ways until either
/// side closes it.
async fn connection(
    stream: TcpStream,
    address: SocketAddr,
    mut closed: watch::Receiver<()>,
    shared: Arc<Shared>,
) {
    let handshake = tokio::select! {
        handshake = timeout(HANDSHAKE_TIMEOUT, accept_async(stream)) => handshake,
        _ = closed.changed() => return,
    };
    let mut socket = match handshake {
        Ok(Ok(socket)) => socket,
        Ok(Err(error)) => {
            let error = error.to_string();
            shared
                .events
                .send(ServerEvent::HandshakeFailed { address, error });
            return;
        }
        Err(_) => {
            let error = "no WebSocket handshake within 10 seconds".to_owned();
            shared
                .events
                .send(ServerEvent::HandshakeFailed { address, error });
            return;
        }
    };

    let (outgoing, mut queue) = mpsc::unbounded_channel();
    let registration = Registration::new(&shared, address, outgoing);
    let mut link = Link {
        registration: &registration,
        unanswered: VecDeque::new(),
        tracked: Tracked::default(),
    };
    // The first tick is immediate: the state is asked for on connect.
    let mut resync = interval(RESYNC_INTERVAL);
    resync.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        let step = tokio::select! {
            received = socket.next() => Step::Received(received),
            Some(message) = queue.recv() => Step::Outgoing(message),
            _ = resync.tick() => Step::Resync,
            _ = closed.changed() => Step::Close,
        };
        let sent = match step {
            Step::Received(Some(Ok(Message::Text(text)))) => {
                link.received(&text);
                Vec::new()
            }
            // Other messages are ignored. Pings are answered by tungstenite
            // itself, and a Close from the timer ends the stream.
            Step::Received(Some(Ok(_))) => Vec::new(),
            Step::Received(Some(Err(_)) | None) => return,
            Step::Outgoing(Outgoing::Action { text, command }) => {
                vec![(text, Pending::Action(command))]
            }
            Step::Outgoing(Outgoing::Resync) | Step::Resync => link.ask_state(),
            Step::Close => {
                let frame = CloseFrame {
                    code: CloseCode::Away,
                    reason: "The server is stopping".into(),
                };
                // Best effort: the timer may already be gone.
                let _ = timeout(CLOSE_TIMEOUT, socket.close(Some(frame))).await;
                return;
            }
        };
        // Questions that fill in the tracked state, after what it just
        // learned.
        let questions = link.tracked.queries().into_iter().map(|query| {
            let text = protocol::encode_query(&query);
            (text, Pending::Query(query))
        });
        for (text, pending) in sent.into_iter().chain(questions) {
            if socket.send(Message::text(text)).await.is_err() {
                return;
            }
            if link.unanswered.len() == MAX_UNANSWERED {
                link.unanswered.pop_front();
            }
            link.unanswered.push_back(pending);
        }
    }
}

/// A connection's side of the conversation with its timer: what it is
/// waiting for answers to, and the timer's tracked state.
struct Link<'a> {
    registration: &'a Registration,
    /// What was sent and not answered yet, oldest first.
    unanswered: VecDeque<Pending>,
    tracked: Tracked,
}

impl Link<'_> {
    /// The question for the timer's state, unless one is already waiting
    /// for its answer.
    fn ask_state(&self) -> Vec<(String, Pending)> {
        let waiting = self
            .unanswered
            .iter()
            .any(|pending| matches!(pending, Pending::Query(Query::State)));
        if waiting {
            return Vec::new();
        }
        vec![(
            protocol::encode_query(&Query::State),
            Pending::Query(Query::State),
        )]
    }

    /// Handles a text message from the timer: an answer to the oldest thing
    /// sent, or an event. A rejected command is reported, but isn't an
    /// error (spec §9).
    fn received(&mut self, text: &str) {
        let before = self.tracked.summary();
        match protocol::parse(text) {
            Received::Success(value) => match self.unanswered.pop_front() {
                Some(Pending::Query(query)) => self.answer(query, &value),
                Some(Pending::Action(_)) | None => {}
            },
            Received::Error { code, message } => match self.unanswered.pop_front() {
                // A question the timer couldn't answer, for example about a
                // segment that is gone after a reset. The next state answer
                // puts things right.
                Some(Pending::Query(_)) => {}
                command => {
                    let command = match command {
                        Some(Pending::Action(command)) => Some(command.to_owned()),
                        _ => None,
                    };
                    let registration = self.registration;
                    registration
                        .shared
                        .events
                        .send(ServerEvent::CommandRejected {
                            id: registration.id,
                            address: registration.address,
                            command,
                            code,
                            message,
                        });
                }
            },
            Received::Event(event) => self.tracked.event(&event),
            Received::Other => {}
        }
        self.publish(before);
    }

    /// Updates the tracked state with the answer to a question.
    fn answer(&mut self, query: Query, value: &serde_json::Value) {
        match query {
            Query::State => {
                if let Some((phase, index)) = protocol::parse_state(value) {
                    self.tracked.state_answer(phase, index);
                }
            }
            Query::SplitTime { index, attempt } => {
                self.tracked
                    .split_time_answer(index, attempt, !value.is_null());
            }
            Query::SegmentName { index, attempt } => {
                if let Some(name) = value.as_str() {
                    self.tracked
                        .segment_name_answer(index, attempt, name.to_owned());
                }
            }
        }
    }

    /// Copies the tracked state into the registry, for the auto splitter's
    /// questions, and reports a change of phase or index.
    fn publish(&self, before: Option<super::Summary>) {
        let registration = self.registration;
        let shared = &registration.shared;
        if let Some(connection) = shared
            .connections()
            .iter_mut()
            .find(|connection| connection.id == registration.id)
        {
            connection.tracked.clone_from(&self.tracked);
        }
        if let Some(summary) = self.tracked.summary()
            && before != Some(summary)
        {
            shared.events.send(ServerEvent::TimerState {
                id: registration.id,
                address: registration.address,
                summary,
            });
        }
    }
}

/// A connected timer's place in the registry. Dropping it, including when
/// its task is aborted, removes it and reports the timer disconnected.
struct Registration {
    shared: Arc<Shared>,
    id: super::ConnectionId,
    address: SocketAddr,
}

impl Registration {
    fn new(
        shared: &Arc<Shared>,
        address: SocketAddr,
        outgoing: mpsc::UnboundedSender<Outgoing>,
    ) -> Self {
        let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
        shared.connections().push(Connection {
            id,
            outgoing,
            tracked: Tracked::default(),
        });
        shared
            .events
            .send(ServerEvent::TimerConnected { id, address });
        Self {
            shared: shared.clone(),
            id,
            address,
        }
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        {
            let mut connections = self.shared.connections();
            let was_primary = connections.first().map(|c| c.id) == Some(self.id);
            connections.retain(|connection| connection.id != self.id);
            // The longest connected timer takes over, and is asked for its
            // state again (spec §5.3).
            if was_primary && let Some(primary) = connections.first() {
                let _ = primary.outgoing.send(Outgoing::Resync);
            }
        }
        self.shared.events.send(ServerEvent::TimerDisconnected {
            id: self.id,
            address: self.address,
        });
    }
}
