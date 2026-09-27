//! Address row: a connection URL with its network label and Copy.

use eframe::egui::{Align, Label, Layout, RichText, Ui, vec2};

use super::{
    Width,
    button::{Button, ButtonSize},
};
use crate::ui::theme;

/// The row's height: the label and the URL, stacked.
const HEIGHT: f32 = 32.0;
/// How long Copy shows "COPIED ✓", in seconds.
const COPIED_FOR: f64 = 1.5;

/// An address row: the network label (LAN or VPN) over the URL, and Copy,
/// which copies the URL and shows "COPIED ✓" in green for 1.5 s.
pub fn address_row(ui: &mut Ui, label: &str, url: &str, width: Width) {
    let size = vec2(ui.available_width(), HEIGHT);
    ui.allocate_ui_with_layout(size, Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        copy_button(ui, url);
        // A top-down child takes the space left of the button, so a long
        // URL is cut short before it.
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.label(
                RichText::new(label)
                    .font(theme::mono(10.0))
                    .color(theme::TEXT_MUTED),
            );
            let url_size = match width {
                Width::Wide => 12.0,
                Width::Compact => 13.0,
            };
            let url = RichText::new(url)
                .font(theme::mono(url_size))
                .color(theme::TEXT);
            ui.add(Label::new(url).truncate());
        });
    });
}

/// Copy, or "COPIED ✓" for a moment after it was clicked.
fn copy_button(ui: &mut Ui, url: &str) {
    let id = ui.make_persistent_id(("copy", url));
    let now = ui.input(|input| input.time);
    let copied_at = ui.data(|data| data.get_temp::<f64>(id));
    let left = copied_at.map_or(0.0, |at| COPIED_FOR - (now - at));

    let button = if left > 0.0 {
        // Repaint when it's time to go back to Copy.
        ui.ctx().request_repaint_after_secs(left as f32);
        Button::secondary("COPIED ✓").text_color(theme::STATUS_OK)
    } else {
        Button::secondary("Copy")
    };
    if ui.add(button.size(ButtonSize::Xs)).clicked() {
        ui.ctx().copy_text(url.to_owned());
        ui.data_mut(|data| data.insert_temp(id, now));
    }
}
