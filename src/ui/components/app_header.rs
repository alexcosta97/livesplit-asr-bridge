//! App header: the wordmark at the top of the status column.

use eframe::egui::{RichText, Ui};

use super::{Width, status_word::display_text};
use crate::{ui::theme, version::VERSION};

/// The app header: the wordmark, with the version when wide. Show details and
/// ← Status come with the tab view (#14, #15).
pub fn app_header(ui: &mut Ui, width: Width) {
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
}
