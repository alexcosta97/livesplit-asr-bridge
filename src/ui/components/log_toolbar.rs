//! Log toolbar: the category filters and the actions on the log.

use eframe::egui::{RichText, Ui, Vec2};

use super::{
    button::{Button, ButtonSize},
    checkbox::Checkbox,
    section_label,
};
use crate::{logging::Filters, ui::theme};

/// What was clicked on the log toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogAction {
    /// A filter was ticked or unticked.
    FiltersChanged,
    Copy,
    Save,
    Clear,
    OpenFolder,
}

/// The log toolbar: SHOW with a checkbox per optional category, the note
/// that errors are always shown, and Copy, Save log…, Clear and Open log
/// folder.
pub fn log_toolbar(ui: &mut Ui, filters: &mut Filters) -> Option<LogAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(16.0, 8.0);
        section_label(ui, "Show");
        let checkboxes = [
            (&mut filters.auto_splitter, "Auto splitter"),
            (&mut filters.connection, "Connection"),
            (&mut filters.app, "App & runtime"),
        ];
        for (checked, label) in checkboxes {
            if ui.add(Checkbox::new(checked, label)).changed() {
                action = Some(LogAction::FiltersChanged);
            }
        }
    });
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.0);
        ui.label(
            RichText::new("Errors are always shown.")
                .font(theme::body(13.0))
                .color(theme::TEXT_MUTED),
        );
        ui.add_space(10.0);
        let buttons = [
            ("Copy", LogAction::Copy),
            ("Save log…", LogAction::Save),
            ("Clear", LogAction::Clear),
            ("Open log folder", LogAction::OpenFolder),
        ];
        for (label, clicked) in buttons {
            if ui
                .add(Button::secondary(label).size(ButtonSize::Sm))
                .clicked()
            {
                action = Some(clicked);
            }
        }
    });
    action
}
