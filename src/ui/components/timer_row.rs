//! Timer row: a connected timer, in the Connection tab.

use std::net::SocketAddr;

use eframe::egui::{Frame, Margin, RichText, Stroke, Ui};

use crate::ui::theme;

/// The row's height.
const HEIGHT: f32 = 48.0;

/// A timer row: the timer's address, like `192.168.1.42:53122`. The tracked
/// state and the PRIMARY tag come with #11.
pub fn timer_row(ui: &mut Ui, address: SocketAddr) {
    Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // The frame's 1 px border is outside the inner height.
            ui.set_height(HEIGHT - 2.0);
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new(address.to_string())
                        .font(theme::mono(13.0))
                        .color(theme::TEXT),
                );
            });
        });
}
