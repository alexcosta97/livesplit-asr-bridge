//! Timer row: a connected timer, in the Connection tab.

use std::net::SocketAddr;

use eframe::egui::{Align, Frame, Layout, Margin, RichText, Stroke, Ui};

use crate::{server::Summary, ui::theme};

/// The row's height.
const HEIGHT: f32 = 48.0;

/// A timer row: the timer's address, like `192.168.1.42:53122`, a PRIMARY
/// tag when the tracked state follows it (spec §5.3), and its tracked state,
/// like "Running · split 12", once it answered.
pub fn timer_row(ui: &mut Ui, address: SocketAddr, state: Option<Summary>, primary: bool) {
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
                if primary {
                    ui.add_space(4.0);
                    primary_tag(ui);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(state) = state {
                        ui.label(
                            RichText::new(state.to_string())
                                .font(theme::body(13.0))
                                .color(theme::TEXT_SECONDARY),
                        );
                    }
                });
            });
        });
}

/// The PRIMARY tag: an orange mono label in a thin orange outline.
fn primary_tag(ui: &mut Ui) {
    Frame::NONE
        .stroke(Stroke::new(1.0, theme::ACCENT))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(
                RichText::new("PRIMARY")
                    .font(theme::mono(11.0))
                    .color(theme::ACCENT)
                    .extra_letter_spacing(0.08 * 11.0),
            );
        });
}

#[cfg(test)]
mod tests {
    use eframe::egui::{CentralPanel, Context, RawInput};

    use super::*;
    use crate::server::Phase;

    /// Lays out the row, returning the text it shows.
    fn shown(state: Option<Summary>, primary: bool) -> String {
        let ctx = Context::default();
        crate::ui::install_theme(&ctx);
        let address = SocketAddr::from(([192, 168, 1, 42], 53122));
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| timer_row(ui, address, state, primary));
        });
        output.textures_delta.clear();
        let mut text = String::new();
        for shape in output.shapes {
            if let eframe::egui::Shape::Text(shape) = shape.shape {
                text.push_str(shape.galley.text());
                text.push('\n');
            }
        }
        text
    }

    #[test]
    fn shows_the_address_the_state_and_the_primary_tag() {
        let running = Summary {
            phase: Phase::Running,
            index: Some(11),
        };
        let text = shown(Some(running), true);
        for expected in ["192.168.1.42:53122", "PRIMARY", "Running · split 12"] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }

    #[test]
    fn another_timer_has_no_tag() {
        let text = shown(None, false);
        assert!(text.contains("192.168.1.42:53122"), "{text}");
        assert!(!text.contains("PRIMARY"), "{text}");
    }
}
