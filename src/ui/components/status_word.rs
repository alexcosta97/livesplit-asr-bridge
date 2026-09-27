//! Status words, like ATTACHED, with their status dot.

use eframe::egui::{
    Color32, FontId, RichText, Sense, Stroke, TextWrapMode, Ui, WidgetText, pos2, vec2,
};

use super::Width;
use crate::ui::theme;

/// The diameter of a status dot.
const DOT_SIZE: f32 = 8.0;

/// A status dot: filled for a status with a colour, outlined for a neutral
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dot {
    Filled,
    Outlined,
}

/// A status word in Archivo Black, 20 px wide or 30 px compact, after its
/// dot. The word wraps when it doesn't fit, and the dot stays level with its
/// first line.
pub fn status_word(ui: &mut Ui, dot: Option<Dot>, word: &str, color: Color32, width: Width) {
    let (size, gap) = match width {
        Width::Wide => (20.0, 8.0),
        Width::Compact => (30.0, 10.0),
    };
    let indent = if dot.is_some() { DOT_SIZE + gap } else { 0.0 };
    let text = display_text(word, size, color).line_height(Some((size * 1.05).round()));
    let galley = WidgetText::from(text).into_galley(
        ui,
        Some(TextWrapMode::Wrap),
        ui.available_width() - indent,
        FontId::default(),
    );
    let (rect, _) = ui.allocate_exact_size(
        vec2(indent + galley.size().x, galley.size().y),
        Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return;
    }
    let first_line = galley
        .rows
        .first()
        .map_or(galley.size().y, |row| row.rect().height());
    let painter = ui.painter();
    if let Some(dot) = dot {
        let center = pos2(rect.left() + DOT_SIZE / 2.0, rect.top() + first_line / 2.0);
        match dot {
            Dot::Filled => {
                painter.circle_filled(center, DOT_SIZE / 2.0, color);
            }
            Dot::Outlined => {
                painter.circle_stroke(center, DOT_SIZE / 2.0 - 0.5, Stroke::new(1.0, color));
            }
        }
    }
    painter.galley(pos2(rect.left() + indent, rect.top()), galley, color);
}

/// Display text: Archivo Black, uppercase, with tight tracking.
pub fn display_text(text: &str, size: f32, color: Color32) -> RichText {
    RichText::new(text.to_uppercase())
        .font(theme::display(size))
        .color(color)
        .extra_letter_spacing(-0.02 * size)
}
