//! Setting text: a free-form text setting, with its default under it while
//! the text differs from it.

use eframe::egui::{Align, Label, Link, Margin, RichText, TextEdit, Ui, vec2};

use super::setting_choice::{FIELD_HEIGHT, setting_label};
use crate::ui::theme;

/// The widest a text field gets: a scene or level name, not a paragraph.
const MAX_WIDTH: f32 = 480.0;

/// What a text setting asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAction {
    /// The text was edited.
    Changed,
    /// Put the default back.
    UseDefault,
}

/// A text setting: its label, then a one-line field with `text`. While the
/// text differs from `default`, "Default: …" and **Use default** show under
/// the field. An empty field shows the hint "Empty": an empty text is a
/// value of its own, not the default. Returns what was done.
pub fn setting_text(
    ui: &mut Ui,
    label: &str,
    tooltip: Option<&str>,
    text: &mut String,
    default: &str,
) -> Option<TextAction> {
    setting_label(ui, label, tooltip);
    let width = ui.available_width().min(MAX_WIDTH);
    let changed = ui
        .add(
            TextEdit::singleline(text)
                .font(theme::body(14.0))
                .hint_text(RichText::new("Empty").color(theme::TEXT_MUTED))
                .desired_width(width)
                .min_size(vec2(0.0, FIELD_HEIGHT))
                .vertical_align(Align::Center)
                .margin(Margin::symmetric(10, 0)),
        )
        .changed();
    let mut action = changed.then_some(TextAction::Changed);
    if text != default {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.add(
                Label::new(
                    RichText::new(format!("Default: {default}"))
                        .font(theme::mono(11.0))
                        .color(theme::TEXT_MUTED),
                )
                .truncate(),
            );
            if ui
                .add(Link::new(
                    RichText::new("Use default").font(theme::body(12.0)),
                ))
                .clicked()
            {
                action = Some(TextAction::UseDefault);
            }
        });
    }
    action
}
