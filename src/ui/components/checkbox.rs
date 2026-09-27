//! Checkbox: a setting or option that is on or off.

use eframe::egui::{
    Label, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
    vec2,
};

use super::info_marker::info_marker;
use crate::ui::theme;

/// The box's size.
const BOX: f32 = 16.0;
/// The height of a checkbox's row.
const ROW_HEIGHT: f32 = 28.0;

/// A checkbox: a 16 px box, orange with a black check when on, and its
/// label, with an info marker when it has a tooltip and an optional note
/// below. Clicking the box or the label toggles it.
pub fn checkbox(
    ui: &mut Ui,
    checked: &mut bool,
    label: &str,
    tooltip: Option<&str>,
    note: Option<&str>,
) -> Response {
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let galley = ui.painter().layout(
                label.to_owned(),
                theme::body(14.0),
                theme::TEXT,
                (ui.available_width() - BOX - 8.0 - 22.0).max(0.0),
            );
            let size = vec2(BOX + 8.0 + galley.size().x, ROW_HEIGHT.max(galley.size().y));
            let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
            if response.clicked() {
                *checked = !*checked;
                response.mark_changed();
            }
            response
                .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, *checked, label));
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                let box_rect = Rect::from_min_size(
                    pos2(rect.left(), rect.center().y - BOX / 2.0),
                    vec2(BOX, BOX),
                );
                let border = if response.hovered() || response.has_focus() {
                    theme::ACCENT
                } else {
                    theme::BORDER_STRONG
                };
                if *checked {
                    painter.rect_filled(box_rect, theme::RADIUS / 2, theme::ACCENT);
                    let check = [
                        box_rect.left_top() + vec2(3.5, 8.0),
                        box_rect.left_top() + vec2(6.5, 11.0),
                        box_rect.left_top() + vec2(12.5, 5.0),
                    ];
                    painter.line(check.to_vec(), Stroke::new(2.0, theme::WINDOW));
                } else {
                    painter.rect(
                        box_rect,
                        theme::RADIUS / 2,
                        theme::PANEL,
                        Stroke::new(1.0, border),
                        StrokeKind::Inside,
                    );
                }
                let text_pos = pos2(
                    box_rect.right() + 8.0,
                    rect.center().y - galley.size().y / 2.0,
                );
                painter.galley(text_pos, galley, theme::TEXT);
            }
            if let Some(tooltip) = tooltip {
                info_marker(ui, tooltip);
            }
            response
        })
        .inner;
    if let Some(note) = note {
        ui.horizontal(|ui| {
            ui.add_space(BOX + 8.0);
            ui.add(
                Label::new(
                    RichText::new(note)
                        .font(theme::body(12.0))
                        .color(theme::TEXT_MUTED),
                )
                .wrap(),
            );
        });
    }
    response
}
