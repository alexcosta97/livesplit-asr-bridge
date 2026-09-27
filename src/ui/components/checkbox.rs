//! Checkbox: a 16 px square, orange with a black check when ticked, its
//! label and an optional note under the label.

use eframe::egui::{
    Color32, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetInfo, WidgetType, pos2,
    vec2,
};

use crate::ui::theme;

/// The size of the square.
const BOX_SIZE: f32 = 16.0;
/// The space between the square and the label.
const GAP: f32 = 12.0;
/// The space between the label and the note.
const NOTE_GAP: f32 = 6.0;

/// A checkbox that ticks and unticks `checked` when clicked.
#[must_use = "add it with `ui.add`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    label: &'a str,
    note: Option<&'a str>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, label: &'a str) -> Self {
        Self {
            checked,
            label,
            note: None,
        }
    }

    /// Adds a note under the label, in muted text that wraps.
    pub fn note(mut self, note: &'a str) -> Self {
        self.note = Some(note);
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let galley =
            ui.painter()
                .layout_no_wrap(self.label.to_owned(), theme::body(14.0), theme::TEXT);
        let note = self.note.map(|note| {
            let wrap_width = (ui.available_width() - BOX_SIZE - GAP).max(0.0);
            ui.painter().layout(
                note.to_owned(),
                theme::body(13.0),
                theme::TEXT_MUTED,
                wrap_width,
            )
        });
        let label_height = BOX_SIZE.max(galley.size().y);
        let note_size = note
            .as_ref()
            .map_or(Vec2::ZERO, |note| note.size() + vec2(0.0, NOTE_GAP));
        let size = vec2(
            BOX_SIZE + GAP + galley.size().x.max(note_size.x),
            label_height + note_size.y,
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
            let label_center = rect.top() + label_height / 2.0;
            let square = eframe::egui::Rect::from_min_size(
                pos2(rect.left(), label_center - BOX_SIZE / 2.0),
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
                3,
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            if checked {
                let at = |x: f32, y: f32| square.min + vec2(x, y) * BOX_SIZE;
                painter.line(
                    vec![at(0.289, 0.523), at(0.43, 0.664), at(0.711, 0.336)],
                    Stroke::new(1.65, theme::WINDOW),
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
            let text_pos = pos2(square.right() + GAP, label_center - galley.size().y / 2.0);
            painter.galley(text_pos, galley, theme::TEXT);
            if let Some(note) = note {
                let note_pos = pos2(text_pos.x, rect.top() + label_height + NOTE_GAP);
                painter.galley(note_pos, note, theme::TEXT_MUTED);
            }
        }
        response
    }
}
