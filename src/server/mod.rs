//! The Server: a WebSocket server that LiveSplit One connects to with
//! Connect to Server.
//!
//! It listens on all interfaces, on a dedicated thread running a
//! single-threaded tokio runtime, and keeps a registry of the connected
//! timers. Everything that happens (listening, a port in use, timers
//! connecting and disconnecting, a command rejected) is reported as a
//! [`ServerEvent`].
//!
//! The auto splitter controls the timers through the [`TimerLink`] from
//! [`Server::link`]: each action becomes a command (spec §5.1), sent to every
//! connected timer through its queue in the registry, which never blocks the
//! auto splitter thread. With no timer connected, the command is dropped.
//!
//! Each connection tracks its timer's state (spec §5.2), and the auto
//! splitter's questions are answered from the primary timer's: the first to
//! connect, then the longest connected once it leaves (spec §5.3).

mod addresses;
mod events;
mod protocol;
mod thread;
mod tracked;

use std::{
    sync::{Arc, Mutex, MutexGuard, atomic::AtomicU64, mpsc::Receiver},
    thread::JoinHandle,
};

use livesplit_auto_splitting::TimerState;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::runner::{TimerAction, TimerLink};

pub use addresses::{Network, NetworkAddress, network_addresses};
pub use events::{ConnectionId, ServerEvent};
#[cfg(test)]
pub use tracked::Phase;
pub use tracked::Summary;

use events::EventSink;
use thread::Command;
use tracked::Tracked;

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
    /// The connected timers, in the order they connected. The first is the
    /// primary timer, whose state the auto splitter's questions are answered
    /// from.
    connections: Mutex<Vec<Connection>>,
    next_id: AtomicU64,
}

/// A connected timer, in the registry.
struct Connection {
    id: ConnectionId,
    /// Commands for this timer. Its task sends them in order.
    outgoing: UnboundedSender<Outgoing>,
    /// A copy of the timer's tracked state, which its task keeps up to date.
    tracked: Tracked,
}

/// Something for a timer's task to send.
enum Outgoing {
    /// A command for the auto splitter's action.
    Action {
        /// The command, as JSON text.
        text: String,
        /// Its name, to say which command a rejection is for.
        command: &'static str,
    },
    /// Ask for the timer's state now: it has become the primary timer.
    Resync,
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

    /// The timer the auto splitter controls: its actions are sent to every
    /// connected timer. It keeps working across restarts.
    pub fn link(&self) -> Arc<ServerLink> {
        Arc::new(ServerLink {
            shared: self.shared.clone(),
        })
    }
}

/// Sends the auto splitter's actions to the connected timers.
pub struct ServerLink {
    shared: Arc<Shared>,
}

impl ServerLink {
    /// Answers from the primary timer's tracked state; with no timer
    /// connected, from a timer that isn't running (spec §5.2).
    fn primary<T>(&self, answer: impl FnOnce(&Tracked) -> T) -> T {
        match self.shared.connections().first() {
            Some(primary) => answer(&primary.tracked),
            None => answer(&Tracked::default()),
        }
    }
}

impl TimerLink for ServerLink {
    fn state(&self) -> TimerState {
        self.primary(Tracked::state)
    }

    fn current_split_index(&self) -> Option<usize> {
        self.primary(Tracked::current_split_index)
    }

    fn segment_splitted(&self, index: usize) -> Option<bool> {
        self.primary(|tracked| tracked.segment_splitted(index))
    }

    fn segment_name(&self) -> Option<String> {
        self.primary(|tracked| {
            if tracked.state() == TimerState::Running {
                tracked.segment_name().map(str::to_owned)
            } else {
                None
            }
        })
    }

    /// Queues the action's command for every connected timer, returning how
    /// many it was queued for. With none, the command is dropped, never
    /// queued for later (spec §5.1).
    fn send(&self, action: TimerAction) -> usize {
        let connections = self.shared.connections();
        if connections.is_empty() {
            return 0;
        }
        let text = protocol::encode(&action);
        let command = protocol::command_name(&action);
        connections
            .iter()
            .filter(|connection| {
                let outgoing = Outgoing::Action {
                    text: text.clone(),
                    command,
                };
                connection.outgoing.send(outgoing).is_ok()
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
pub mod fake_timer;
#[cfg(test)]
mod tests;
