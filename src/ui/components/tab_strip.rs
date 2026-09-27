//! Tab strip: the tabs above the tab area, the active one underlined in
//! orange, with a `•` on a tab that has unsaved changes.

use std::sync::Arc;

use eframe::egui::{
    Color32, Galley, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};

use crate::ui::theme;

/// The strip's height.
const HEIGHT: f32 = 40.0;
/// The space between two tabs.
const GAP: f32 = 24.0;
/// The space between a label and its `•`.
const DOT_GAP: f32 = 6.0;
/// The label's font size.
const FONT_SIZE: f32 = 12.0;

/// A tab in the strip.
pub struct Tab<'a, T> {
    pub id: T,
    pub label: &'a str,
    /// Draws a `•` after the label, for unsaved changes.
    pub unsaved: bool,
}

/// The tab strip, filling the width: Inter SemiBold 12 px uppercase labels,
/// a 1 px line below, and a 2 px orange underline under the active tab.
/// Clicking a tab makes it `active`.
pub fn tab_strip<T: Copy + PartialEq>(ui: &mut Ui, tabs: &[Tab<'_, T>], active: &mut T) {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.painter().hline(
        strip.x_range(),
        strip.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    let mut x = strip.left();
    for tab in tabs {
        let label = tab.label.to_uppercase();
        // Coloured when painted, so hover can change it.
        let text = layout(ui, &label);
        let dot = tab.unsaved.then(|| layout(ui, "•"));
        let width = text.size().x + dot.as_ref().map_or(0.0, |dot| DOT_GAP + dot.size().x);
        let rect = Rect::from_min_max(pos2(x, strip.top()), pos2(x + width, strip.bottom()));
        x += width + GAP;

        let response = ui.interact(rect, ui.id().with(("tab", &label)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, *active == tab.id, &label)
        });
        if response.clicked() {
            *active = tab.id;
        }
        if !ui.is_rect_visible(rect) {
            continue;
        }
        let is_active = *active == tab.id;
        let color = if is_active || response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_SECONDARY
        };
        let painter = ui.painter();
        let text_pos = pos2(rect.left(), rect.center().y - text.size().y / 2.0);
        let text_width = text.size().x;
        painter.galley(text_pos, text, color);
        if let Some(dot) = dot {
            painter.galley(
                pos2(text_pos.x + text_width + DOT_GAP, text_pos.y),
                dot,
                theme::STATUS_WARNING,
            );
        }
        if is_active {
            painter.hline(
                rect.x_range(),
                rect.bottom() - 1.0,
                Stroke::new(2.0, theme::ACCENT),
            );
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand2(vec2(6.0, -6.0)),
                theme::RADIUS,
                Stroke::new(1.0, theme::ACCENT),
                StrokeKind::Outside,
            );
        }
    }
}

/// A tab label, with +0.06em tracking.
fn layout(ui: &Ui, text: &str) -> Arc<Galley> {
    let format = TextFormat {
        font_id: theme::body_semibold(FONT_SIZE),
        extra_letter_spacing: 0.06 * FONT_SIZE,
        color: Color32::PLACEHOLDER,
        ..Default::default()
    };
    ui.painter()
        .layout_job(LayoutJob::single_section(text.to_owned(), format))
}
