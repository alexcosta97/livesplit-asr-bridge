//! Section label: the small uppercase label at the top of a card or section.

use eframe::egui::{Response, RichText, Ui};

use crate::ui::theme;

/// A section label: small uppercase mono text, muted, with wide tracking.
pub fn section_label(ui: &mut Ui, text: &str) -> Response {
    ui.label(
        RichText::new(text.to_uppercase())
            .font(theme::mono(11.0))
            .color(theme::TEXT_MUTED)
            .extra_letter_spacing(0.08 * 11.0),
    )
}
