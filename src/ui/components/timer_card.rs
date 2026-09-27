//! Timer card: whether LiveSplit One is connected, and where to connect.

use eframe::egui::{RichText, Sense, Ui, vec2};

use super::{
    Width,
    address_row::address_row,
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
/// or server stopped. "How do I connect?" and `?` come with #15.
pub fn timer_card(ui: &mut Ui, state: TimerState, urls: &[(Network, String)], width: Width) {
    let edge = matches!(state, TimerState::Connected { .. }).then_some(Edge {
        color: theme::STATUS_OK,
        tint: theme::TINT_OK,
    });
    card(ui, edge, |ui| {
        section_label(ui, "Timer");
        ui.add_space(8.0);
        match state {
            TimerState::Connected { timers } => {
                status_word(ui, Some(Dot::Filled), "Connected", theme::STATUS_OK, width);
                ui.add_space(4.0);
                detail(ui, &timer_count(timers));
                if width == Width::Wide {
                    ui.add_space(12.0);
                    detail(ui, "Addresses: Connection tab");
                }
            }
            TimerState::NotConnected => {
                status_word(
                    ui,
                    Some(Dot::Outlined),
                    "Not connected",
                    theme::STATUS_WAITING,
                    width,
                );
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
    });
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
