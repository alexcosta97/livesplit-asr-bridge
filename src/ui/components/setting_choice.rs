//! Setting choice and Setting file: a dropdown, and a path with Browse….
//! A free-form text setting uses the same field.

use eframe::egui::{
    Align, ComboBox, Frame, Label, Layout, Margin, Response, RichText, Stroke, TextEdit, Ui, vec2,
};

use super::{button::Button, info_marker::info_marker};
use crate::ui::{settings::ChoiceOption, theme};

/// The height of a field, as tall as a button.
const FIELD_HEIGHT: f32 = 32.0;
/// The width of a dropdown.
const CHOICE_WIDTH: f32 = 240.0;

/// A setting's label, with an info marker when it has a tooltip.
fn setting_label(ui: &mut Ui, label: &str, tooltip: Option<&str>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add(
            Label::new(
                RichText::new(label)
                    .font(theme::body(14.0))
                    .color(theme::TEXT),
            )
            .wrap(),
        );
        if let Some(tooltip) = tooltip {
            info_marker(ui, tooltip);
        }
    });
    ui.add_space(6.0);
}

/// A choice: its label, then a dropdown of `options`. Returns the key of
/// the option picked, if one was.
pub fn setting_choice(
    ui: &mut Ui,
    id: &str,
    label: &str,
    tooltip: Option<&str>,
    options: &[ChoiceOption],
    selected: &str,
) -> Option<String> {
    setting_label(ui, label, tooltip);
    let shown = options
        .iter()
        .find(|option| &*option.key == selected)
        .map_or(selected, |option| &option.description);
    let mut picked = None;
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = FIELD_HEIGHT;
        ui.spacing_mut().button_padding = vec2(10.0, 0.0);
        ComboBox::from_id_salt(("setting choice", id))
            .width(CHOICE_WIDTH)
            .selected_text(RichText::new(shown).font(theme::body(14.0)))
            .show_ui(ui, |ui| {
                for option in options {
                    let is_selected = &*option.key == selected;
                    if ui
                        .selectable_label(
                            is_selected,
                            RichText::new(&*option.description).font(theme::body(14.0)),
                        )
                        .clicked()
                        && !is_selected
                    {
                        picked = Some(option.key.to_string());
                    }
                }
            });
    });
    picked
}

/// A file selection: its label, then the path in a mono field and
/// **Browse…**. Returns whether Browse… was clicked.
pub fn setting_file(ui: &mut Ui, label: &str, tooltip: Option<&str>, path: Option<&str>) -> bool {
    setting_label(ui, label, tooltip);
    // Right to left, so Browse… keeps its size and the path takes the rest.
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let browse = ui.add(Button::secondary("Browse…"));
        Frame::NONE
            .fill(theme::PANEL)
            .stroke(Stroke::new(1.0, theme::BORDER))
            .corner_radius(theme::RADIUS)
            .inner_margin(Margin::symmetric(10, 0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_height(FIELD_HEIGHT - 2.0);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let (text, color) = match path {
                        Some(path) => (path, theme::TEXT),
                        None => ("No file selected", theme::TEXT_MUTED),
                    };
                    let label = ui.add(
                        Label::new(RichText::new(text).font(theme::mono(12.0)).color(color))
                            .truncate(),
                    );
                    if let Some(path) = path {
                        label.on_hover_text(path);
                    }
                });
            });
        browse.clicked()
    })
    .inner
}

/// A free-form text setting: its label, then a text field.
pub fn setting_text(
    ui: &mut Ui,
    label: &str,
    tooltip: Option<&str>,
    text: &mut String,
) -> Response {
    setting_label(ui, label, tooltip);
    ui.add(
        TextEdit::singleline(text)
            .font(theme::body(14.0))
            .desired_width(f32::INFINITY)
            .min_size(vec2(0.0, FIELD_HEIGHT))
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(10, 0)),
    )
}
