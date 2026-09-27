//! App header: the wordmark at the top of the status column.

use eframe::egui::{Align, Layout, RichText, Ui};

use super::{
    Width,
    button::{Button, ButtonSize},
    status_word::display_text,
};
use crate::{ui::theme, version::VERSION};

/// The app header: the wordmark, with the version when wide. Over the
/// compact tab view it has ← Status at the right, and returns whether it was
/// clicked. Show details (spec §6.3) is not built yet.
pub fn app_header(ui: &mut Ui, width: Width, back: bool) -> bool {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let size = match width {
                Width::Wide => 14.0,
                Width::Compact => 13.0,
            };
            ui.label(display_text("LiveSplit One ASR Bridge", size, theme::TEXT));
            if width == Width::Wide {
                ui.label(
                    RichText::new(VERSION)
                        .font(theme::mono(11.0))
                        .color(theme::TEXT_MUTED),
                );
            }
        });
        back && ui
            .with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Button::secondary("← Status").size(ButtonSize::Xs))
                    .clicked()
            })
            .inner
    })
    .inner
}
