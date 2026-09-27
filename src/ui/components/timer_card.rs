//! Timer card: whether LiveSplit One is connected, and where to connect.

use eframe::egui::{Link, RichText, Sense, Ui, vec2};

use super::{
    Width,
    address_row::address_row,
    button::{Button, ButtonSize},
    card::{Edge, card},
    section_label::section_label,
    status_word::{Dot, status_word},
};
use crate::{
    server::Network,
    ui::{
        server_status::{TimerState, timer_count},
        theme,
    },
};

/// The Timer card: connected with the timer count, not connected with the
/// addresses to connect to (all of them when wide, the first when compact),
/// or server stopped. Not connected, it has "How do I connect?"; connected
/// or not, a `?`. Returns whether either was clicked, to open the setup
/// steps.
pub fn timer_card(
    ui: &mut Ui,
    state: TimerState,
    urls: &[(Network, String)],
    width: Width,
) -> bool {
    let edge = matches!(state, TimerState::Connected { .. }).then_some(Edge {
        color: theme::STATUS_OK,
        tint: theme::TINT_OK,
    });
    card(ui, edge, |ui| {
        let mut steps = false;
        section_label(ui, "Timer");
        ui.add_space(8.0);
        match state {
            TimerState::Connected { timers } => {
                status_word(ui, Some(Dot::Filled), "Connected", theme::STATUS_OK, width);
                ui.add_space(4.0);
                detail(ui, &timer_count(timers));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    detail(ui, "Addresses: Connection tab");
                    steps |= help_button(ui);
                });
            }
            TimerState::NotConnected => {
                status_word(
                    ui,
                    Some(Dot::Outlined),
                    "Not connected",
                    theme::STATUS_WAITING,
                    width,
                );
                ui.add_space(4.0);
                steps |= ui
                    .add(Link::new(
                        RichText::new("How do I connect?").font(theme::body(13.0)),
                    ))
                    .clicked();
                ui.add_space(12.0);
                divider(ui);
                ui.add_space(12.0);
                let shown = match width {
                    Width::Wide => urls,
                    Width::Compact => &urls[..urls.len().min(1)],
                };
                if shown.is_empty() {
                    help(ui, "No network connection found");
                }
                for (index, (network, url)) in shown.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(8.0);
                    }
                    address_row(ui, network.label(), url, width);
                }
                ui.add_space(8.0);
                steps |= help_button(ui);
            }
            TimerState::ServerStopped => {
                status_word(
                    ui,
                    Some(Dot::Outlined),
                    "Server stopped",
                    theme::STATUS_WAITING,
                    width,
                );
                ui.add_space(4.0);
                help(ui, "No timer can connect until the server restarts");
            }
        }
        steps
    })
    .inner
}

/// The `?` that opens the setup steps. Returns whether it was clicked.
fn help_button(ui: &mut Ui) -> bool {
    ui.add(Button::secondary("?").size(ButtonSize::Xs))
        .on_hover_text("How to connect")
        .clicked()
}

/// A mono detail line, like the timer count.
fn detail(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::mono(12.0))
            .color(theme::TEXT_SECONDARY),
    );
}

/// A help line, like the Game card's.
fn help(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .font(theme::body(13.0))
            .color(theme::TEXT_SECONDARY),
    );
}

/// A 1 px line across the card.
fn divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::BORDER);
}

#[cfg(test)]
mod tests {
    use eframe::egui::{CentralPanel, Context, RawInput, accesskit::Role};

    use super::*;

    /// The buttons on the card, and the link, as "button: …" and
    /// "link: …". egui reports a link to accessibility as a label, so the
    /// link is found by its text.
    fn controls(state: TimerState, width: Width) -> Vec<String> {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let urls = [(Network::Lan, "ws://192.168.1.20:16834".to_owned())];
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| timer_card(ui, state, &urls, width));
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        let mut controls: Vec<String> = output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_, node)| {
                let label = node.label()?;
                match node.role() {
                    Role::Button => Some(format!("button: {label}")),
                    Role::Label if label == "How do I connect?" => Some(format!("link: {label}")),
                    _ => None,
                }
            })
            .collect();
        controls.sort();
        controls
    }

    #[test]
    fn not_connected_has_the_link_and_the_question_mark() {
        for width in [Width::Wide, Width::Compact] {
            assert_eq!(
                controls(TimerState::NotConnected, width),
                ["button: ?", "button: Copy", "link: How do I connect?"],
                "{width:?}"
            );
        }
    }

    #[test]
    fn connected_has_only_the_question_mark() {
        for width in [Width::Wide, Width::Compact] {
            assert_eq!(
                controls(TimerState::Connected { timers: 1 }, width),
                ["button: ?"],
                "{width:?}"
            );
        }
    }

    #[test]
    fn server_stopped_has_nothing_to_connect_to() {
        assert!(controls(TimerState::ServerStopped, Width::Wide).is_empty());
    }
}
