//! The Server: a WebSocket server that LiveSplit One connects to with
//! Connect to Server.
//!
//! It listens on all interfaces, on a dedicated thread running a
//! single-threaded tokio runtime, and keeps a registry of the connected
//! timers. Everything that happens (listening, a port in use, timers
//! connecting and disconnecting) is reported as a [`ServerEvent`].
//!
//! Sending commands to the timers, and tracking their state, come later
//! (#10, #11): the registry already holds a queue per connection that any
//! thread can push to without blocking, which is what the auto splitter
//! thread needs.

mod addresses;
mod events;
mod thread;

use std::{
    sync::{Arc, Mutex, MutexGuard, atomic::AtomicU64, mpsc::Receiver},
    thread::JoinHandle,
};

use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

pub use addresses::{Network, NetworkAddress, network_addresses};
pub use events::{ConnectionId, ServerEvent};

use events::EventSink;
use thread::Command;

/// Runs the WebSocket server. Dropping it closes every connection, stops
/// listening and waits for its thread to finish.
pub struct Server {
    commands: UnboundedSender<Command>,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

/// What the Server thread and the handle share.
struct Shared {
    events: EventSink,
    /// The connected timers, in the order they connected.
    connections: Mutex<Vec<Connection>>,
    next_id: AtomicU64,
}

/// A connected timer, in the registry.
struct Connection {
    id: ConnectionId,
    /// Messages for this timer. Its task sends them in order.
    outgoing: UnboundedSender<Message>,
}

impl Server {
    /// Starts the Server thread, listening on `port` on all interfaces; 0
    /// picks a free port, reported in the `Listening` event. Events arrive on
    /// the returned receiver, and `wake` is called after each one, for
    /// example to repaint the window.
    pub fn new(
        port: u16,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> (Self, Receiver<ServerEvent>) {
        let (events, receiver) = EventSink::new(wake);
        let shared = Arc::new(Shared {
            events,
            connections: Mutex::default(),
            next_id: AtomicU64::new(1),
        });
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let thread = std::thread::Builder::new()
            .name("server".into())
            .spawn({
                let shared = shared.clone();
                move || thread::run(port, command_receiver, shared)
            })
            .expect("failed to start the server thread");
        let server = Self {
            commands,
            shared,
            thread: Some(thread),
        };
        (server, receiver)
    }

    /// Closes every connection, stops listening and listens again on `port`.
    /// Works whether or not the server is listening, so it also retries
    /// after a port was in use.
    pub fn restart(&self, port: u16) {
        let _ = self.commands.send(Command::Restart(port));
    }

    /// Queues a text message for every connected timer, returning how many
    /// it was queued for. Never blocks, so the auto splitter thread can call
    /// it.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "timer commands are sent with it from #10")
    )]
    pub fn send_to_all(&self, text: &str) -> usize {
        let connections = self.shared.connections();
        connections
            .iter()
            .filter(|connection| {
                connection
                    .outgoing
                    .send(Message::text(text.to_owned()))
                    .is_ok()
            })
            .count()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Shared {
    fn connections(&self) -> MutexGuard<'_, Vec<Connection>> {
        // Nothing panics while holding the lock, but recover anyway rather
        // than take the app down.
        self.connections
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests;
