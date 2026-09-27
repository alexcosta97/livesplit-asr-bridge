//! The Server thread: a single-threaded tokio runtime that accepts timers
//! and runs one task per connection.

use std::{
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
    time::{sleep, timeout},
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{
        Message,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};

use super::{Connection, ServerEvent, Shared};

/// How long a new connection has to complete the WebSocket handshake, so a
/// connection that never does doesn't linger.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// How long closing connections may take, on restart and shutdown, before
/// they are cut off.
const CLOSE_TIMEOUT: Duration = Duration::from_millis(500);
/// How long to wait after a failed accept, for example when the process is
/// out of file descriptors, so the loop doesn't spin.
const ACCEPT_RETRY: Duration = Duration::from_millis(100);

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
    Outgoing(Message),
    Close,
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
    let _registration = Registration::new(&shared, address, outgoing);

    loop {
        let step = tokio::select! {
            received = socket.next() => Step::Received(received),
            Some(message) = queue.recv() => Step::Outgoing(message),
            _ = closed.changed() => Step::Close,
        };
        match step {
            // Timers' messages are handled from #11. Pings are answered by
            // tungstenite itself, and a Close from the timer ends the stream.
            Step::Received(Some(Ok(_))) => {}
            Step::Received(Some(Err(_)) | None) => return,
            Step::Outgoing(message) => {
                if socket.send(message).await.is_err() {
                    return;
                }
            }
            Step::Close => {
                let frame = CloseFrame {
                    code: CloseCode::Away,
                    reason: "The server is stopping".into(),
                };
                // Best effort: the timer may already be gone.
                let _ = timeout(CLOSE_TIMEOUT, socket.close(Some(frame))).await;
                return;
            }
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
        outgoing: mpsc::UnboundedSender<Message>,
    ) -> Self {
        let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
        shared.connections().push(Connection { id, outgoing });
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
        self.shared
            .connections()
            .retain(|connection| connection.id != self.id);
        self.shared.events.send(ServerEvent::TimerDisconnected {
            id: self.id,
            address: self.address,
        });
    }
}
