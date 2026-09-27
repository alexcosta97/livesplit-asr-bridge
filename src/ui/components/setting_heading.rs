//! Setting heading: a heading in the settings list, level 1 or 2.

use eframe::egui::{Label, RichText, Ui};

use super::info_marker::info_marker;
use crate::ui::theme;

/// A heading in the settings list. The runtime's top level, `0`, is drawn as
/// level 1: small uppercase mono text. Deeper levels are level 2: Inter
/// SemiBold in the secondary colour. With a tooltip, an info marker follows.
pub fn setting_heading(ui: &mut Ui, text: &str, level: u32, tooltip: Option<&str>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let text = if level == 0 {
            RichText::new(text.to_uppercase())
                .font(theme::mono(11.0))
                .color(theme::TEXT_MUTED)
                .extra_letter_spacing(0.08 * 11.0)
        } else {
            RichText::new(text)
                .font(theme::body_semibold(13.0))
                .color(theme::TEXT_SECONDARY)
        };
        ui.add(Label::new(text).wrap());
        if let Some(tooltip) = tooltip {
            info_marker(ui, tooltip);
        }
    });
}
