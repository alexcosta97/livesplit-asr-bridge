//! Checkbox: a 16 px square, orange with a black check when ticked, and its
//! label.

use eframe::egui::{
    Color32, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::ui::theme;

/// The size of the square.
const BOX_SIZE: f32 = 16.0;
/// The space between the square and the label.
const GAP: f32 = 8.0;

/// A checkbox that ticks and unticks `checked` when clicked.
#[must_use = "add it with `ui.add`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: &'a str,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str) -> Self {
        Self { checked, label }
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let galley =
            ui.painter()
                .layout_no_wrap(self.label.to_owned(), theme::body(13.0), theme::TEXT);
        let size = vec2(
            BOX_SIZE + GAP + galley.size().x,
            BOX_SIZE.max(galley.size().y),
        );
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
        if response.clicked() {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        let checked = *self.checked;
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, checked, self.label));

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let square = eframe::egui::Rect::from_min_size(
                pos2(rect.left(), rect.center().y - BOX_SIZE / 2.0),
                vec2(BOX_SIZE, BOX_SIZE),
            );
            let (fill, border) = if checked {
                (theme::ACCENT, theme::ACCENT)
            } else if response.hovered() {
                (theme::RAISED, theme::BORDER_STRONG)
            } else {
                (Color32::TRANSPARENT, theme::BORDER_STRONG)
            };
            painter.rect(
                square,
                2,
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            if checked {
                let at = |x: f32, y: f32| square.min + vec2(x, y) * BOX_SIZE;
                painter.line(
                    vec![at(0.25, 0.52), at(0.43, 0.7), at(0.76, 0.33)],
                    Stroke::new(2.0, theme::WINDOW),
                );
            }
            if response.has_focus() {
                painter.rect_stroke(
                    square.expand(2.0),
                    4,
                    Stroke::new(1.0, theme::ACCENT),
                    StrokeKind::Outside,
                );
            }
            let text_pos = pos2(
                square.right() + GAP,
                rect.center().y - galley.size().y / 2.0,
            );
            painter.galley(text_pos, galley, theme::TEXT);
        }
        response
    }
}
