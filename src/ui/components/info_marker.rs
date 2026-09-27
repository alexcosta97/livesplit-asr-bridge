//! Info marker and Tooltip: the `i` next to a setting, and the tooltip it
//! opens on hover.

use eframe::egui::{
    Frame, Label, Margin, Popup, PopupKind, Response, RichText, Sense, Stroke, Ui, pos2, vec2,
};

use crate::ui::theme;

/// The marker's diameter.
const SIZE: f32 = 14.0;
/// The widest a tooltip grows before its text wraps.
const TOOLTIP_WIDTH: f32 = 280.0;

/// An `i` in an outlined circle, drawn as shapes rather than a font glyph,
/// that shows `text` in a tooltip while hovered.
pub fn info_marker(ui: &mut Ui, text: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(SIZE, SIZE), Sense::hover());
    let hovered = response.hovered();
    if ui.is_rect_visible(rect) {
        let color = if hovered {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_MUTED
        };
        let painter = ui.painter();
        let center = rect.center();
        painter.circle_stroke(center, SIZE / 2.0 - 0.5, Stroke::new(1.0, color));
        painter.circle_filled(pos2(center.x, center.y - 3.0), 1.0, color);
        painter.line_segment(
            [
                pos2(center.x, center.y - 0.5),
                pos2(center.x, center.y + 3.5),
            ],
            Stroke::new(1.5, color),
        );
    }
    tooltip(&response, text, hovered);
    response
}

/// The tooltip: a small raised box with the text, next to `response`.
fn tooltip(response: &Response, text: &str, open: bool) {
    let frame = Frame::NONE
        .fill(theme::RAISED)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::symmetric(10, 8));
    Popup::from_response(response)
        .kind(PopupKind::Tooltip)
        .open(open)
        .frame(frame)
        .show(|ui| {
            ui.set_max_width(TOOLTIP_WIDTH);
            ui.add(
                Label::new(
                    RichText::new(text)
                        .font(theme::body(12.0))
                        .color(theme::TEXT),
                )
                .wrap(),
            );
        });
}
