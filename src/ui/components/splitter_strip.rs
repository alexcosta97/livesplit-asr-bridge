//! Splitter strip: the compact state's one-line Auto splitter card.

use eframe::egui::{Align, Label, Layout, RichText, Ui, vec2};

use super::{
    SplitterAction,
    button::{Button, ButtonSize},
    reload_hint,
};
use crate::ui::theme;

/// The strip's height: the height of its button.
const HEIGHT: f32 = 24.0;

/// The Splitter strip: the loaded file with Reload, or "No auto splitter
/// loaded" with Open….
pub fn splitter_strip(ui: &mut Ui, loaded: Option<&str>) -> Option<SplitterAction> {
    let size = vec2(ui.available_width(), HEIGHT);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let action = match loaded {
            Some(_) => reload_hint(ui.add(Button::secondary("Reload").size(ButtonSize::Xs)))
                .clicked()
                .then_some(SplitterAction::Reload),
            None => ui
                .add(Button::primary("Open…").size(ButtonSize::Xs))
                .clicked()
                .then_some(SplitterAction::Open),
        };
        // A left-to-right child takes the space left of the button, so the
        // text starts at the left edge and is cut short before the button.
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let (text, color) = match loaded {
                Some(file_name) => (file_name, theme::TEXT_SECONDARY),
                None => ("No auto splitter loaded", theme::TEXT_MUTED),
            };
            let text = RichText::new(text).font(theme::mono(12.0)).color(color);
            ui.add(Label::new(text).truncate());
        });
        action
    })
    .inner
}
