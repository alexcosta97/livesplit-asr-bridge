//! Empty state: what the Settings tab shows with no auto splitter loaded.

use eframe::egui::{Label, RichText, Ui};

use super::button::Button;
use crate::ui::theme;

/// "No auto splitter loaded", how to load one, and **Open…**. Returns
/// whether Open… was clicked.
pub fn empty_state(ui: &mut Ui) -> bool {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(
            RichText::new("NO AUTO SPLITTER LOADED")
                .font(theme::mono(13.0))
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(8.0);
        ui.add(
            Label::new(
                RichText::new(
                    "Its settings appear here once it is loaded. Open a .wasm auto splitter to \
                     start.",
                )
                .font(theme::body(14.0))
                .color(theme::TEXT_SECONDARY),
            )
            .wrap(),
        );
        ui.add_space(16.0);
        ui.add(Button::primary("Open…")).clicked()
    })
    .inner
}
