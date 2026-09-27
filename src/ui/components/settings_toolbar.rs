//! Settings toolbar: Save, Revert to defaults and the save status.

use eframe::egui::{RichText, Ui};

use super::button::Button;
use crate::ui::theme;

/// What the Settings toolbar shows next to its buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStatus {
    /// Nothing to save.
    Nothing,
    /// There are edits to save.
    Unsaved,
    /// The settings were just saved.
    Saved,
}

/// What was clicked on the Settings toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    Save,
    RevertToDefaults,
}

/// The Settings toolbar: **Save**, enabled only when there is something to
/// save, **Revert to defaults**, and "● Unsaved changes" or "✓ Saved".
pub fn settings_toolbar(ui: &mut Ui, status: SaveStatus) -> Option<ToolbarAction> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let save = ui.add(Button::primary("Save").enabled(status == SaveStatus::Unsaved));
        let revert = ui
            .add(Button::secondary("Revert to defaults"))
            .on_hover_text("Put back the auto splitter's default values. Save to use them.");
        ui.add_space(8.0);
        let text = match status {
            SaveStatus::Nothing => None,
            SaveStatus::Unsaved => Some(("● Unsaved changes", theme::STATUS_WARNING)),
            SaveStatus::Saved => Some(("✓ Saved", theme::STATUS_OK)),
        };
        if let Some((text, color)) = text {
            ui.label(RichText::new(text).font(theme::body(13.0)).color(color));
        }
        if save.clicked() {
            Some(ToolbarAction::Save)
        } else if revert.clicked() {
            Some(ToolbarAction::RevertToDefaults)
        } else {
            None
        }
    })
    .inner
}
