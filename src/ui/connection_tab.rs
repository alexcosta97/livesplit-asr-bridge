//! The Connection tab: the server's port, the connected timers and how to
//! connect.

use eframe::egui::{RichText, ScrollArea, Ui};

use super::{
    components::{self, PortNote},
    server_status::ServerStatus,
    theme,
};
use crate::server::Network;

/// The space between the tab's sections.
const SECTION_GAP: f32 = 32.0;

/// The Connection tab's own state: the port being edited, and the port the
/// server was last started with. The app saves that port in `app.toml` when
/// Restart server is pressed.
pub struct ConnectionTab {
    port_text: String,
    applied_port: u16,
}

impl ConnectionTab {
    /// The tab for a server started on `port`.
    pub fn new(port: u16) -> Self {
        Self {
            port_text: port.to_string(),
            applied_port: port,
        }
    }

    /// The port in the field, if it is one: 1 to 65535.
    fn edited_port(&self) -> Option<u16> {
        self.port_text.trim().parse().ok().filter(|port| *port != 0)
    }

    /// How the port in the field compares with the one the server was
    /// started with.
    pub fn port_note(&self) -> PortNote {
        match self.edited_port() {
            None => PortNote::Invalid,
            Some(port) if port == self.applied_port => PortNote::Applied,
            Some(_) => PortNote::Edited,
        }
    }

    /// Takes the port in the field as the one to restart on, returning it.
    /// When the field doesn't hold a port, it keeps the port the server was
    /// started with and shows it again.
    pub fn apply_port(&mut self) -> u16 {
        if let Some(port) = self.edited_port() {
            self.applied_port = port;
        }
        self.port_text = self.applied_port.to_string();
        self.applied_port
    }

    /// Shows the tab. Returns the port to restart the server on, when
    /// Restart server was pressed.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        status: &ServerStatus,
        urls: &[(Network, String)],
    ) -> Option<u16> {
        let mut restart = None;
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let error = status.bind_error().map(|error| error.message());
            let note = self.port_note();
            if components::server_section(ui, &mut self.port_text, note, error.as_deref()) {
                restart = Some(self.apply_port());
            }

            ui.add_space(SECTION_GAP);
            components::section_label(ui, "Connected timers");
            ui.add_space(16.0);
            if status.timers().is_empty() {
                ui.label(
                    RichText::new("No timers connected yet.")
                        .font(theme::body(13.0))
                        .color(theme::TEXT_MUTED),
                );
            }
            for (index, timer) in status.timers().iter().enumerate() {
                if index > 0 {
                    ui.add_space(8.0);
                }
                components::timer_row(ui, timer.address);
            }

            ui.add_space(SECTION_GAP);
            components::how_to_connect(ui, urls, status.listening_port().is_some());
        });
        restart
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(text: &str) -> ConnectionTab {
        let mut tab = ConnectionTab::new(16834);
        tab.port_text = text.to_owned();
        tab
    }

    #[test]
    fn the_started_port_is_applied() {
        assert_eq!(ConnectionTab::new(16834).port_note(), PortNote::Applied);
        assert_eq!(tab(" 16834 ").port_note(), PortNote::Applied);
    }

    #[test]
    fn another_port_is_edited() {
        assert_eq!(tab("16835").port_note(), PortNote::Edited);
        assert_eq!(tab("1").port_note(), PortNote::Edited);
        assert_eq!(tab("65535").port_note(), PortNote::Edited);
    }

    #[test]
    fn anything_else_is_invalid() {
        for text in ["", "0", "65536", "-1", "abc", "16834a"] {
            assert_eq!(tab(text).port_note(), PortNote::Invalid, "{text:?}");
        }
    }

    #[test]
    fn restart_applies_an_edited_port() {
        let mut tab = tab("9000");
        assert_eq!(tab.apply_port(), 9000);
        assert_eq!(tab.port_note(), PortNote::Applied);
    }

    #[test]
    fn restart_with_an_invalid_port_keeps_the_started_one() {
        let mut tab = tab("99999");
        assert_eq!(tab.apply_port(), 16834);
        assert_eq!(tab.port_text, "16834");
        assert_eq!(tab.port_note(), PortNote::Applied);
    }
}
