//! Log line: one line of the log, coloured by category.

use eframe::egui::{
    Align, Color32, Frame, Label, Layout, Margin, RichText, Stroke, TextWrapMode, Ui, vec2,
};

use crate::{
    logging::{Category, Entry},
    ui::theme,
};

/// The height of a line's text.
const TEXT_HEIGHT: f32 = 20.0;
/// The width of the outline, drawn round every line so a highlighted one is
/// the same height.
const OUTLINE: f32 = 1.0;
/// The height of a log line, outline included.
pub const LOG_LINE_HEIGHT: f32 = TEXT_HEIGHT + 2.0 * OUTLINE;
/// The width of the category tag, so messages line up.
const TAG_WIDTH: f32 = 112.0;
/// The width of the time.
const TIME_WIDTH: f32 = 96.0;

/// A log line: the time (muted), the category tag in its colour, and the
/// message, in Space Mono 12 px. A highlighted line has a 1 px orange
/// outline over a raised background.
pub fn log_line(ui: &mut Ui, entry: &Entry, highlighted: bool) {
    let (fill, outline) = if highlighted {
        (theme::RAISED, theme::ACCENT)
    } else {
        (Color32::TRANSPARENT, Color32::TRANSPARENT)
    };
    let frame = Frame::NONE
        .fill(fill)
        .stroke(Stroke::new(OUTLINE, outline))
        .corner_radius(2);
    frame.inner_margin(Margin::symmetric(4, 0)).show(ui, |ui| {
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), TEXT_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let mono =
                    |text: &str, color| RichText::new(text).font(theme::mono(12.0)).color(color);
                cell(ui, TIME_WIDTH, mono(&entry.clock(), theme::TEXT_MUTED));
                cell(
                    ui,
                    TAG_WIDTH,
                    mono(entry.category.tag(), tag_color(entry.category)),
                );
                ui.add(
                    Label::new(mono(&entry.message, theme::TEXT)).wrap_mode(TextWrapMode::Extend),
                );
            },
        );
    });
}

/// A fixed-width cell, so the columns line up.
fn cell(ui: &mut Ui, width: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        vec2(width, TEXT_HEIGHT),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_min_width(width);
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
