//! The card frame the status column's cards share.

use eframe::egui::{Color32, Frame, InnerResponse, Margin, Rect, Stroke, Ui, pos2};

use crate::ui::theme;

/// The width of a status edge, on the left of a card.
const EDGE_WIDTH: f32 = 3.0;
/// The padding inside a card.
const PADDING: i8 = 16;

/// A status edge: a 3 px left edge in a status colour, over a tinted
/// background.
pub struct Edge {
    pub color: Color32,
    pub tint: Color32,
}

/// A card: 1 px border, 4 px corners and 16 px padding, filling the width,
/// with an optional status edge. Inside, items have no spacing; each card
/// sets its own gaps.
pub fn card<R>(
    ui: &mut Ui,
    edge: Option<Edge>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let fill = edge.as_ref().map_or(theme::WINDOW, |edge| edge.tint);
    // The edge replaces the 1 px border on the left, so the content moves
    // right by the difference, as in the mockups.
    let extra_left = if edge.is_some() {
        EDGE_WIDTH as i8 - 1
    } else {
        0
    };
    let response = Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin {
            left: PADDING + extra_left,
            ..Margin::same(PADDING)
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add_contents(ui)
        });
    if let Some(edge) = edge {
        let rect = response.response.rect;
        let strip = Rect::from_min_max(rect.min, pos2(rect.left() + EDGE_WIDTH, rect.bottom()));
        // The left of the card's own rounded shape, so the edge follows the
        // rounded corners like a CSS border.
        ui.painter()
            .with_clip_rect(strip.intersect(ui.clip_rect()))
            .rect_filled(rect, theme::RADIUS, edge.color);
    }
    response
}
