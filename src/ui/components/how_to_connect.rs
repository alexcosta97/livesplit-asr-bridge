//! How to connect: the setup steps, in the Connection tab.

use eframe::egui::{
    Align, Align2, Frame, Label, Layout, Margin, RichText, Sense, Stroke, TextFormat, Ui,
    text::LayoutJob, vec2,
};

use super::{Width, address_row::address_row, section_label::section_label};
use crate::{server::Network, ui::theme};

/// The diameter of a step's number.
const NUMBER_SIZE: f32 = 24.0;
/// The width of the box holding the addresses.
const ADDRESSES_WIDTH: f32 = 280.0;
/// The steps' font size.
const FONT_SIZE: f32 = 14.0;

/// How to connect: three numbered steps, the first with every address and
/// its Copy, and the note that LiveSplit One must run in a Chrome-based
/// browser. `listening` is whether the server listens, to explain an empty
/// address list. Collapsing, and opening at these steps, come with #15.
pub fn how_to_connect(ui: &mut Ui, urls: &[(Network, String)], listening: bool) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 16.0;
        section_label(ui, "How to connect");
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 18.0;
            step(ui, 1, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.label(
                    RichText::new("Copy an address")
                        .font(theme::body_semibold(FONT_SIZE))
                        .color(theme::TEXT),
                );
                addresses(ui, urls, listening);
            });
            step(ui, 2, |ui| {
                let regular = TextFormat::simple(theme::body(FONT_SIZE), theme::TEXT);
                let bold = TextFormat::simple(theme::body_semibold(FONT_SIZE), theme::TEXT);
                let mut job = LayoutJob::default();
                job.append("In LiveSplit One, open ", 0.0, regular.clone());
                job.append("Settings → Connect to Server", 0.0, bold);
                job.append(" and paste it.", 0.0, regular);
                ui.add(Label::new(job).wrap());
            });
            step(ui, 3, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.add(
                    Label::new(
                        RichText::new("When Chrome asks, allow local network access.")
                            .font(theme::body(FONT_SIZE))
                            .color(theme::TEXT),
                    )
                    .wrap(),
                );
                ui.add(
                    Label::new(
                        RichText::new("LiveSplit One must run in a Chrome-based browser.")
                            .font(theme::body(13.0))
                            .color(theme::TEXT_MUTED),
                    )
                    .wrap(),
                );
            });
        });
    });
}

/// A numbered step: the number in a circle, then its content.
fn step(ui: &mut Ui, number: u8, add_contents: impl FnOnce(&mut Ui)) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let (rect, _) = ui.allocate_exact_size(vec2(NUMBER_SIZE, NUMBER_SIZE), Sense::hover());
        let painter = ui.painter();
        painter.circle_filled(rect.center(), NUMBER_SIZE / 2.0, theme::RAISED);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            number.to_string(),
            theme::mono(12.0),
            theme::TEXT,
        );
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), 0.0),
            Layout::top_down(Align::Min),
            |ui| {
                // Level with the number's centre, as in the mockup.
                ui.add_space(2.0);
                add_contents(ui);
            },
        );
    });
}

/// The box of addresses, each with Copy.
fn addresses(ui: &mut Ui, urls: &[(Network, String)], listening: bool) {
    let width = ADDRESSES_WIDTH.min(ui.available_width());
    Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            // The frame's padding and border are outside the inner width.
            ui.set_width(width - 26.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            if urls.is_empty() {
                let text = if listening {
                    "No network connection found."
                } else {
                    "The server isn't running. Restart it above."
                };
                ui.add(
                    Label::new(
                        RichText::new(text)
                            .font(theme::body(13.0))
                            .color(theme::TEXT_MUTED),
                    )
                    .wrap(),
                );
            }
            for (network, url) in urls {
                address_row(ui, network.label(), url, Width::Wide);
            }
        });
}
