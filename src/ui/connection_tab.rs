//! The Connection tab: the server's port, the connected timers and how to
//! connect.

use eframe::egui::{Align, RichText, ScrollArea, Ui, style::ScrollAnimation};

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
/// Restart server is pressed. It also holds whether How to connect is
/// collapsed, and whether the tab scrolls to it the next time it is shown.
pub struct ConnectionTab {
    port_text: String,
    applied_port: u16,
    steps_collapsed: bool,
    scroll_to_steps: bool,
}

impl ConnectionTab {
    /// The tab for a server started on `port`.
    pub fn new(port: u16) -> Self {
        Self {
            port_text: port.to_string(),
            applied_port: port,
            steps_collapsed: false,
            scroll_to_steps: false,
        }
    }

    /// Opens the tab at How to connect, expanded, the next time it is
    /// shown: for "How do I connect?", the Timer card's `?` and the first
    /// launch (spec §6.2, §6.9).
    pub fn show_steps(&mut self) {
        self.steps_collapsed = false;
        self.scroll_to_steps = true;
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
                // The first to connect is the one the tracked state follows.
                components::timer_row(ui, timer.address, timer.state, index == 0);
            }

            ui.add_space(SECTION_GAP);
            // The steps can be collapsed only while a timer is connected.
            let collapsed = (!status.timers().is_empty()).then_some(&mut self.steps_collapsed);
            let steps =
                components::how_to_connect(ui, urls, status.listening_port().is_some(), collapsed);
            if std::mem::take(&mut self.scroll_to_steps) {
                // The tab has just opened, so it starts there rather than
                // scrolling.
                steps.scroll_to_me_animation(Some(Align::Min), ScrollAnimation::none());
            }
        });
        restart
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{self, Context, RawInput, Rect, vec2};

    use super::*;
    use crate::server::ServerEvent;

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

    /// The server's status: listening, with `timers` timers connected.
    fn listening(timers: u64) -> ServerStatus {
        let mut status = ServerStatus::default();
        status.apply(&ServerEvent::Listening { port: 16834 });
        for id in 0..timers {
            status.apply(&ServerEvent::TimerConnected {
                id,
                address: ([192, 168, 1, 42], 53122).into(),
            });
        }
        status
    }

    /// Draws the tab in a window `height` tall, returning the labels on
    /// screen.
    fn draw(
        ctx: &Context,
        tab: &mut ConnectionTab,
        status: &ServerStatus,
        height: f32,
    ) -> Vec<String> {
        theme::install(ctx);
        ctx.enable_accesskit();
        let screen = Rect::from_min_size(egui::Pos2::ZERO, vec2(500.0, height));
        let input = RawInput {
            screen_rect: Some(screen),
            ..RawInput::default()
        };
        let urls = [(Network::Lan, "ws://192.168.1.20:16834".to_owned())];
        let mut output = ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| tab.show(ui, status, &urls));
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, node)| {
                node.bounds()
                    .is_some_and(|bounds| bounds.y0 < f64::from(height) && bounds.y1 > 0.0)
            })
            .filter_map(|(_, node)| node.label().or(node.value()).map(str::to_owned))
            .collect()
    }

    #[test]
    fn show_steps_scrolls_to_how_to_connect() {
        let ctx = Context::default();
        let status = listening(0);
        let mut tab = ConnectionTab::new(16834);
        let on_screen = draw(&ctx, &mut tab, &status, 160.0);
        assert!(
            !on_screen.iter().any(|text| text == "HOW TO CONNECT"),
            "{on_screen:?}"
        );

        tab.show_steps();
        // The scroll area reaches its target over the next frames.
        for _ in 0..3 {
            draw(&ctx, &mut tab, &status, 160.0);
        }
        let on_screen = draw(&ctx, &mut tab, &status, 160.0);
        assert!(
            on_screen.iter().any(|text| text == "HOW TO CONNECT"),
            "{on_screen:?}"
        );
    }

    #[test]
    fn the_steps_collapse_only_while_a_timer_is_connected() {
        let ctx = Context::default();
        let mut tab = ConnectionTab::new(16834);
        let on_screen = draw(&ctx, &mut tab, &listening(0), 2000.0);
        assert!(
            !on_screen.iter().any(|text| text == "Hide steps"),
            "{on_screen:?}"
        );

        let on_screen = draw(&ctx, &mut tab, &listening(1), 2000.0);
        assert!(
            on_screen.iter().any(|text| text == "Hide steps"),
            "{on_screen:?}"
        );
        assert!(on_screen.iter().any(|text| text == "Copy an address"));

        tab.steps_collapsed = true;
        let on_screen = draw(&ctx, &mut tab, &listening(1), 2000.0);
        assert!(
            on_screen.iter().any(|text| text == "Show steps"),
            "{on_screen:?}"
        );
        assert!(!on_screen.iter().any(|text| text == "Copy an address"));

        // Opening at the steps expands them.
        tab.show_steps();
        let on_screen = draw(&ctx, &mut tab, &listening(1), 2000.0);
        assert!(on_screen.iter().any(|text| text == "Copy an address"));
    }
}
