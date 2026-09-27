//! App header: the wordmark at the top of the status column.

use eframe::egui::{Align, Layout, RichText, Ui};

use super::{
    Width,
    button::{Button, ButtonSize},
    status_word::display_text,
};
use crate::{ui::theme, version::VERSION};

/// The button at the right of the header in the compact state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderButton {
    /// Show details, over the status column.
    ShowDetails,
    /// ← Status, over the tab view.
    Back,
}

impl HeaderButton {
    fn label(self) -> &'static str {
        match self {
            Self::ShowDetails => "Show details",
            Self::Back => "← Status",
        }
    }
}

/// The app header: the wordmark, with the version when wide. In the compact
/// state it has `button` at the top right, and returns whether it was
/// clicked.
pub fn app_header(ui: &mut Ui, width: Width, button: Option<HeaderButton>) -> bool {
    ui.horizontal_top(|ui| {
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
        let button = button.filter(|_| width == Width::Compact);
        button.is_some_and(|button| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add(Button::secondary(button.label()).size(ButtonSize::Xs))
                    .clicked()
            })
            .inner
        })
    })
    .inner
}

#[cfg(test)]
mod tests {
    use eframe::egui::{CentralPanel, Context, RawInput, accesskit::Role};

    use super::*;

    /// The labels of the buttons on the header.
    fn buttons(width: Width, button: Option<HeaderButton>) -> Vec<String> {
        let ctx = Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| app_header(ui, width, button));
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, node)| node.role() == Role::Button)
            .filter_map(|(_, node)| node.label().map(str::to_owned))
            .collect()
    }

    #[test]
    fn compact_has_its_button_and_wide_has_none() {
        let show_details = Some(HeaderButton::ShowDetails);
        assert_eq!(buttons(Width::Compact, show_details), ["Show details"]);
        assert_eq!(
            buttons(Width::Compact, Some(HeaderButton::Back)),
            ["← Status"]
        );
        assert!(buttons(Width::Wide, show_details).is_empty());
    }
}
