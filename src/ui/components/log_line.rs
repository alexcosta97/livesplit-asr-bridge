//! Log line: one line of the log, coloured by category.

use eframe::egui::{
    Align, Color32, Frame, Label, Layout, Margin, RichText, Stroke, TextWrapMode, Ui, vec2,
};

use crate::{
    logging::{Category, Entry},
    ui::theme,
};

/// The height of a message line: Space Mono 12 px at a line height of 1.9.
const TEXT_HEIGHT: f32 = 23.0;
/// The height of a line of the trace under a message, at a line height of
/// 1.6.
const TRACE_HEIGHT: f32 = 19.0;
/// The width of the outline, drawn round every line so a highlighted one
/// lines up with the others.
const OUTLINE: f32 = 1.0;
/// The padding inside a highlighted line, above and below its text.
const HIGHLIGHT_PADDING: i8 = 6;
/// The space round a highlighted line, above and below it.
const HIGHLIGHT_SPACE: f32 = 4.0;
/// The padding inside every line, left and right of its text. The log list
/// is widened by as much, so the lines' text lines up with the toolbar.
pub const LOG_LINE_INSET: f32 = 8.0;
/// The space between the time, the category tag and the message.
const GAP: f32 = 16.0;
/// The width of the time.
const TIME_WIDTH: f32 = 92.0;
/// The width of the category tag, so messages line up.
const TAG_WIDTH: f32 = 110.0;

/// The height of `entry`'s log line, spacing included: one line for the
/// message, one per line of its trace, and the highlight's padding.
pub fn log_line_height(entry: &Entry, highlighted: bool) -> f32 {
    let (_, trace) = split(&entry.message);
    let mut height = TEXT_HEIGHT + trace.len() as f32 * TRACE_HEIGHT + 2.0 * OUTLINE;
    if highlighted {
        height += 2.0 * (f32::from(HIGHLIGHT_PADDING) + HIGHLIGHT_SPACE);
    }
    height
}

/// A log line: the time (muted), the category tag in its colour, and the
/// message, in Space Mono 12 px. The lines after a message's first, such as
/// a backtrace, are its trace, muted below it. A highlighted line has a 1 px
/// orange outline over a raised background.
pub fn log_line(ui: &mut Ui, entry: &Entry, highlighted: bool) {
    let (message, trace) = split(&entry.message);
    let (fill, outline, padding, space) = if highlighted {
        (
            theme::RAISED,
            theme::ACCENT,
            HIGHLIGHT_PADDING,
            HIGHLIGHT_SPACE,
        )
    } else {
        (Color32::TRANSPARENT, Color32::TRANSPARENT, 0, 0.0)
    };
    ui.add_space(space);
    Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(OUTLINE, outline))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::symmetric(LOG_LINE_INSET as i8, padding))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(GAP, 0.0);
                let mono =
                    |text: &str, color| RichText::new(text).font(theme::mono(12.0)).color(color);
                cell(ui, TIME_WIDTH, mono(&entry.clock(), theme::TEXT_MUTED));
                cell(
                    ui,
                    TAG_WIDTH,
                    mono(entry.category.tag(), tag_color(entry.category)),
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    text_line(ui, TEXT_HEIGHT, mono(message, theme::TEXT));
                    for line in &trace {
                        text_line(ui, TRACE_HEIGHT, mono(line, theme::TEXT_MUTED));
                    }
                });
            });
        });
    ui.add_space(space);
}

/// The first line of `message`, and the lines after it.
fn split(message: &str) -> (&str, Vec<&str>) {
    let mut lines = message.trim_end().lines();
    let first = lines.next().unwrap_or_default();
    (first, lines.collect())
}

/// One line of text, `height` high, never wrapped.
fn text_line(ui: &mut Ui, height: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        vec2(0.0, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_height(height);
            ui.add(Label::new(text).wrap_mode(TextWrapMode::Extend));
        },
    );
}

/// A fixed-width cell, so the columns line up.
fn cell(ui: &mut Ui, width: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        vec2(width, TEXT_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_size(vec2(width, TEXT_HEIGHT));
            ui.add(Label::new(text).wrap_mode(TextWrapMode::Truncate));
        },
    );
}

/// The colour of a category's tag.
fn tag_color(category: Category) -> Color32 {
    match category {
        Category::Error => theme::STATUS_ERROR,
        Category::AutoSplitter => theme::ACCENT,
        Category::Connection => theme::LOG_CONNECTION,
        Category::App => theme::TEXT_SECONDARY,
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::{CentralPanel, Context, RawInput};

    use super::*;

    fn entry(message: &str) -> Entry {
        Entry {
            id: 0,
            time: chrono::NaiveDate::from_ymd_opt(2026, 9, 27)
                .unwrap()
                .and_hms_milli_opt(19, 31, 8, 992)
                .unwrap(),
            category: Category::Error,
            message: message.to_owned(),
        }
    }

    /// The height the line takes when drawn.
    fn drawn_height(entry: &Entry, highlighted: bool) -> f32 {
        let ctx = Context::default();
        theme::install(&ctx);
        let mut height = 0.0;
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                let top = ui.cursor().top();
                log_line(ui, entry, highlighted);
                height = ui.cursor().top() - top;
            });
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        height
    }

    #[test]
    fn the_height_is_the_height_drawn() {
        let crash = "The auto splitter stopped because of an error: wasm trap\n    \
                     at update (wasm function 42)\n    at read_tags (wasm function 57)\n";
        for message in ["Split", crash] {
            for highlighted in [false, true] {
                let entry = entry(message);
                assert_eq!(
                    drawn_height(&entry, highlighted),
                    log_line_height(&entry, highlighted),
                    "{message:?} {highlighted}"
                );
            }
        }
    }

    #[test]
    fn the_lines_after_the_first_are_the_trace() {
        assert_eq!(split("Split"), ("Split", vec![]));
        assert_eq!(
            split("stopped: trap\n  at a\n  at b\n"),
            ("stopped: trap", vec!["  at a", "  at b"])
        );
    }
}
