//! Log toolbar: the category filters and the actions on the log.

use eframe::egui::{RichText, Stroke, Ui, Vec2};

use super::{button::Button, checkbox::Checkbox, section_label};
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

/// The log toolbar: SHOW with a checkbox per optional category and the note
/// that errors are always shown, then Copy, Save log…, Clear and Open log
/// folder, over a 1 px rule.
pub fn log_toolbar(ui: &mut Ui, filters: &mut Filters) -> Option<LogAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(20.0, 8.0);
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
        ui.label(
            RichText::new("Errors are always shown.")
                .font(theme::body(13.0))
                .color(theme::TEXT_MUTED),
        );
    });
    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(8.0);
        let buttons = [
            ("Copy", LogAction::Copy),
            ("Save log…", LogAction::Save),
            ("Clear", LogAction::Clear),
            ("Open log folder", LogAction::OpenFolder),
        ];
        for (label, clicked) in buttons {
            if ui.add(Button::secondary(label)).clicked() {
                action = Some(clicked);
            }
        }
    });
    ui.add_space(16.0);
    let rule = ui.available_rect_before_wrap().x_range();
    let y = ui.cursor().top() + 0.5;
    ui.painter().hline(rule, y, Stroke::new(1.0, theme::BORDER));
    ui.add_space(1.0);
    action
}
