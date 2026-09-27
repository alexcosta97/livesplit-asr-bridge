//! What the Server reports.

use std::{
    net::SocketAddr,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
};

/// Events are dropped when this many are waiting, so a flood of connections
/// can't use unbounded memory while the window isn't reading them.
const CAPACITY: usize = 10_000;

/// Identifies a connection for as long as the Server runs. Ids are never
/// reused, even across restarts.
pub type ConnectionId = u64;

/// Something that happened in the Server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    /// The server listens for timers on all interfaces, on `port`: the port
    /// actually bound, which differs from the one asked for only when that
    /// was 0.
    Listening { port: u16 },
    /// The server couldn't listen on `port`, and isn't listening at all
    /// until it is restarted. `in_use` is whether another program has the
    /// port.
    BindFailed {
        port: u16,
        error: String,
        in_use: bool,
    },
    /// A timer finished the WebSocket handshake.
    TimerConnected {
        id: ConnectionId,
        address: SocketAddr,
    },
    /// A timer's connection closed, from either side.
    TimerDisconnected {
        id: ConnectionId,
        address: SocketAddr,
    },
    /// Something connected but didn't complete a WebSocket handshake, for
    /// example a port scanner or a web browser asking for a page. Not an
    /// error: the server keeps running.
    HandshakeFailed { address: SocketAddr, error: String },
}

impl ServerEvent {
    /// A one-line description for people.
    pub fn describe(&self) -> String {
        match self {
            Self::Listening { port } => format!("Server listening on port {port}"),
            Self::BindFailed {
                port, in_use: true, ..
            } => format!("Couldn't start the server: port {port} is already in use"),
            Self::BindFailed { port, error, .. } => {
                format!("Couldn't start the server on port {port}: {error}")
            }
            Self::TimerConnected { address, .. } => format!("Timer connected from {address}"),
            Self::TimerDisconnected { address, .. } => {
                format!("Timer disconnected ({address})")
            }
            Self::HandshakeFailed { address, error } => {
                format!("Ignored a connection from {address} that isn't a timer: {error}")
            }
        }
    }

    /// Whether this is an error to show to people.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::BindFailed { .. })
    }
}

/// Sends events to the receiver, then wakes it.
#[derive(Clone)]
pub(super) struct EventSink {
    sender: SyncSender<ServerEvent>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl EventSink {
    pub(super) fn new(wake: impl Fn() + Send + Sync + 'static) -> (Self, Receiver<ServerEvent>) {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let sink = Self {
            sender,
            wake: Arc::new(wake),
        };
        (sink, receiver)
    }

    pub(super) fn send(&self, event: ServerEvent) {
        // A full or closed channel drops the event: nobody is reading.
        if self.sender.try_send(event).is_ok() {
            (self.wake)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address() -> SocketAddr {
        "192.168.1.42:53122".parse().unwrap()
    }

    #[test]
    fn only_bind_failures_are_errors() {
        let bind_failed = ServerEvent::BindFailed {
            port: 16834,
            error: "Address in use".to_owned(),
            in_use: true,
        };
        assert!(bind_failed.is_error());
        let others = [
            ServerEvent::Listening { port: 16834 },
            ServerEvent::TimerConnected {
                id: 1,
                address: address(),
            },
            ServerEvent::TimerDisconnected {
                id: 1,
                address: address(),
            },
            ServerEvent::HandshakeFailed {
                address: address(),
                error: "timed out".to_owned(),
            },
        ];
        for event in others {
            assert!(!event.is_error(), "{event:?}");
        }
    }

    #[test]
    fn descriptions_name_the_port_and_address() {
        assert_eq!(
            ServerEvent::Listening { port: 16834 }.describe(),
            "Server listening on port 16834"
        );
        assert_eq!(
            ServerEvent::BindFailed {
                port: 16834,
                error: "Address in use (os error 98)".to_owned(),
                in_use: true,
            }
            .describe(),
            "Couldn't start the server: port 16834 is already in use"
        );
        assert_eq!(
            ServerEvent::TimerConnected {
                id: 1,
                address: address(),
            }
            .describe(),
            "Timer connected from 192.168.1.42:53122"
        );
    }
}
