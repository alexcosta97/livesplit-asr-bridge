//! Tab strip: the tabs above the tab area, with a `•` on a tab that has
//! unsaved changes.

use eframe::egui::{
    Color32, Rect, Sense, Stroke, StrokeKind, TextFormat, Ui, WidgetInfo, WidgetType, pos2,
    text::LayoutJob, vec2,
};

use crate::ui::theme;

/// The strip's height.
const HEIGHT: f32 = 44.0;
/// The space before the first tab and between tabs.
const GAP: f32 = 24.0;
/// The active tab's underline.
const UNDERLINE: f32 = 2.0;
/// The tab labels' font size.
const FONT_SIZE: f32 = 12.0;
/// The space between a label and its `•`.
const DOT_GAP: f32 = 6.0;

/// The tab strip: uppercase labels, the active one underlined in orange,
/// over a 1 px line across the tab area. Clicking a tab makes it `active`.
/// Each tab is its id, its label, and whether it has unsaved changes, which
/// draws an amber `•` after the label.
pub fn tab_strip<T: Copy + PartialEq>(ui: &mut Ui, tabs: &[(T, &str, bool)], active: &mut T) {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.painter().rect_filled(
        Rect::from_min_max(pos2(strip.left(), strip.bottom() - 1.0), strip.max),
        0.0,
        theme::BORDER,
    );

    let mut left = strip.left() + GAP;
    for &(tab, label, unsaved) in tabs {
        let selected = tab == *active;
        let job = LayoutJob::single_section(
            label.to_uppercase(),
            TextFormat {
                font_id: theme::body_semibold(FONT_SIZE),
                extra_letter_spacing: 0.06 * FONT_SIZE,
                color: Color32::PLACEHOLDER,
                ..TextFormat::default()
            },
        );
        let galley = ui.painter().layout_job(job);
        let dot = unsaved.then(|| {
            ui.painter().layout_no_wrap(
                "•".to_owned(),
                theme::body_semibold(FONT_SIZE),
                theme::STATUS_WARNING,
            )
        });
        let width = galley.size().x + dot.as_ref().map_or(0.0, |dot| DOT_GAP + dot.size().x);
        let rect = Rect::from_min_size(pos2(left, strip.top()), vec2(width, HEIGHT));
        left = rect.right() + GAP;

        let response = ui.interact(rect, ui.id().with(("tab", label)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label)
        });
        if response.clicked() {
            *active = tab;
        }

        let color = if selected || response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_SECONDARY
        };
        let text_pos = pos2(rect.left(), rect.center().y - galley.size().y / 2.0);
        let painter = ui.painter();
        let text_width = galley.size().x;
        painter.galley(text_pos, galley, color);
        if let Some(dot) = dot {
            let dot_pos = pos2(text_pos.x + text_width + DOT_GAP, text_pos.y);
            painter.galley(dot_pos, dot, theme::STATUS_WARNING);
        }
        if selected {
            let underline =
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - UNDERLINE), rect.max);
            painter.rect_filled(underline, 0.0, theme::ACCENT);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand2(vec2(4.0, -8.0)),
                theme::RADIUS,
                Stroke::new(1.0, theme::ACCENT),
                StrokeKind::Outside,
            );
        }
    }
}
