//! What the Timer card and the Connection tab show about the server,
//! worked out from the Server's events.

use std::net::SocketAddr;

use crate::server::{ConnectionId, Network, NetworkAddress, ServerEvent};

/// The state the Timer card shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// At least this many timers are connected.
    Connected { timers: usize },
    /// The server listens, but no timer is connected. The card lists the
    /// addresses to connect to.
    NotConnected,
    /// The server isn't listening, so there is nothing to connect to.
    ServerStopped,
}

/// A connected timer, as the Connection tab lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectedTimer {
    pub id: ConnectionId,
    pub address: SocketAddr,
}

/// Why the server isn't listening, as the Connection tab shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindError {
    pub port: u16,
    pub error: String,
    pub in_use: bool,
}

impl BindError {
    /// The text shown in the Server section.
    pub fn message(&self) -> String {
        if self.in_use {
            format!(
                "Couldn't start the server: port {} is already in use by another program. \
                 Choose another port and press Restart server.",
                self.port
            )
        } else {
            format!(
                "Couldn't start the server on port {}: {}",
                self.port, self.error
            )
        }
    }
}

/// The server's status, updated from each [`ServerEvent`].
#[derive(Debug, Default)]
pub struct ServerStatus {
    /// The port the server listens on, while it listens.
    listening: Option<u16>,
    /// The connected timers, in the order they connected.
    timers: Vec<ConnectedTimer>,
    /// Why the server isn't listening, after it failed to.
    bind_error: Option<BindError>,
}

impl ServerStatus {
    /// Updates the status with an event.
    pub fn apply(&mut self, event: &ServerEvent) {
        match event {
            ServerEvent::Listening { port } => {
                self.listening = Some(*port);
                // A restart closed every connection before listening again.
                self.timers.clear();
                self.bind_error = None;
            }
            ServerEvent::BindFailed {
                port,
                error,
                in_use,
            } => {
                self.listening = None;
                self.timers.clear();
                self.bind_error = Some(BindError {
                    port: *port,
                    error: error.clone(),
                    in_use: *in_use,
                });
            }
            ServerEvent::TimerConnected { id, address } => self.timers.push(ConnectedTimer {
                id: *id,
                address: *address,
            }),
            ServerEvent::TimerDisconnected { id, .. } => {
                self.timers.retain(|timer| timer.id != *id);
            }
            ServerEvent::HandshakeFailed { .. } | ServerEvent::CommandRejected { .. } => {}
        }
    }

    /// The port the server listens on, while it listens.
    pub fn listening_port(&self) -> Option<u16> {
        self.listening
    }

    /// The connected timers, in the order they connected.
    pub fn timers(&self) -> &[ConnectedTimer] {
        &self.timers
    }

    /// Why the server isn't listening, after it failed to.
    pub fn bind_error(&self) -> Option<&BindError> {
        self.bind_error.as_ref()
    }

    /// The state the Timer card shows.
    pub fn timer_state(&self) -> TimerState {
        match (self.listening, self.timers.len()) {
            (None, _) => TimerState::ServerStopped,
            (Some(_), 0) => TimerState::NotConnected,
            (Some(_), timers) => TimerState::Connected { timers },
        }
    }

    /// The connection URL of each address, on the port the server listens
    /// on. None while the server isn't listening: an edited port that wasn't
    /// applied is never shown.
    pub fn urls(&self, addresses: &[NetworkAddress]) -> Vec<(Network, String)> {
        let Some(port) = self.listening else {
            return Vec::new();
        };
        addresses
            .iter()
            .map(|address| (address.network, address.url(port)))
            .collect()
    }
}

/// The line under CONNECTED, like "2 timers · LiveSplit One".
pub fn timer_count(timers: usize) -> String {
    let noun = if timers == 1 { "timer" } else { "timers" };
    format!("{timers} {noun} · LiveSplit One")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(port: u16) -> SocketAddr {
        SocketAddr::from(([192, 168, 1, 42], port))
    }

    fn connected(id: ConnectionId) -> ServerEvent {
        ServerEvent::TimerConnected {
            id,
            address: address(50_000 + id as u16),
        }
    }

    fn disconnected(id: ConnectionId) -> ServerEvent {
        ServerEvent::TimerDisconnected {
            id,
            address: address(50_000 + id as u16),
        }
    }

    fn port_in_use(port: u16) -> ServerEvent {
        ServerEvent::BindFailed {
            port,
            error: "Address already in use (os error 98)".to_owned(),
            in_use: true,
        }
    }

    fn status(events: &[ServerEvent]) -> ServerStatus {
        let mut status = ServerStatus::default();
        for event in events {
            status.apply(event);
        }
        status
    }

    fn lan(ip: [u8; 4]) -> NetworkAddress {
        NetworkAddress {
            network: Network::Lan,
            ip: ip.into(),
        }
    }

    #[test]
    fn stopped_until_listening() {
        let status = ServerStatus::default();
        assert_eq!(status.timer_state(), TimerState::ServerStopped);
        assert_eq!(status.listening_port(), None);
    }

    #[test]
    fn not_connected_while_listening_without_timers() {
        let status = status(&[ServerEvent::Listening { port: 16834 }]);
        assert_eq!(status.timer_state(), TimerState::NotConnected);
        assert_eq!(status.listening_port(), Some(16834));
    }

    #[test]
    fn counts_connected_timers_in_connect_order() {
        let mut status = status(&[
            ServerEvent::Listening { port: 16834 },
            connected(3),
            connected(1),
        ]);
        assert_eq!(status.timer_state(), TimerState::Connected { timers: 2 });
        let ids: Vec<_> = status.timers().iter().map(|timer| timer.id).collect();
        assert_eq!(ids, [3, 1]);
        assert_eq!(status.timers()[0].address, address(50_003));

        status.apply(&disconnected(3));
        assert_eq!(status.timer_state(), TimerState::Connected { timers: 1 });
        status.apply(&disconnected(1));
        assert_eq!(status.timer_state(), TimerState::NotConnected);
    }

    #[test]
    fn an_unknown_disconnect_changes_nothing() {
        let status = status(&[
            ServerEvent::Listening { port: 16834 },
            connected(1),
            disconnected(7),
        ]);
        assert_eq!(status.timer_state(), TimerState::Connected { timers: 1 });
    }

    #[test]
    fn port_in_use_stops_the_server() {
        let status = status(&[
            ServerEvent::Listening { port: 16834 },
            connected(1),
            port_in_use(16835),
        ]);
        assert_eq!(status.timer_state(), TimerState::ServerStopped);
        assert!(status.timers().is_empty());
        let error = status.bind_error().unwrap();
        assert_eq!(error.port, 16835);
        assert_eq!(
            error.message(),
            "Couldn't start the server: port 16835 is already in use by another program. \
             Choose another port and press Restart server."
        );
    }

    #[test]
    fn listening_again_clears_the_bind_error() {
        let status = status(&[port_in_use(16834), ServerEvent::Listening { port: 16835 }]);
        assert_eq!(status.bind_error(), None);
        assert_eq!(status.listening_port(), Some(16835));
    }

    #[test]
    fn urls_use_the_listening_port() {
        let addresses = [lan([192, 168, 1, 20])];
        let status = status(&[ServerEvent::Listening { port: 16835 }]);
        assert_eq!(
            status.urls(&addresses),
            [(Network::Lan, "ws://192.168.1.20:16835".to_owned())]
        );
    }

    #[test]
    fn no_urls_while_not_listening() {
        let addresses = [lan([192, 168, 1, 20])];
        assert!(ServerStatus::default().urls(&addresses).is_empty());
        let status = status(&[ServerEvent::Listening { port: 16834 }, port_in_use(16835)]);
        assert!(status.urls(&addresses).is_empty());
    }

    #[test]
    fn timer_count_is_singular_for_one() {
        assert_eq!(timer_count(1), "1 timer · LiveSplit One");
        assert_eq!(timer_count(2), "2 timers · LiveSplit One");
    }
}
