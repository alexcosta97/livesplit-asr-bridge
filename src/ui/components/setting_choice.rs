//! Setting choice and Setting file: a dropdown, and a path with ✕ and
//! Browse….

use eframe::egui::{
    Align, Color32, ComboBox, Frame, Label, Layout, Margin, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
};

use super::{button::Button, info_marker::info_marker};
use crate::ui::{settings::ChoiceOption, theme};

/// The height of a field, as tall as a button.
pub(super) const FIELD_HEIGHT: f32 = 32.0;
/// The width of a dropdown.
const CHOICE_WIDTH: f32 = 240.0;

/// A setting's label, with an info marker when it has a tooltip.
pub(super) fn setting_label(ui: &mut Ui, label: &str, tooltip: Option<&str>) {
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

/// What a file selection asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAction {
    /// Pick a file.
    Browse,
    /// Forget the file picked.
    Clear,
}

/// A file selection: its label, then the path in a mono field, ✕ to clear
/// it when there is one, and **Browse…**, then the names of its filters in
/// a muted mono line when it has some. Returns what was clicked.
pub fn setting_file(
    ui: &mut Ui,
    label: &str,
    tooltip: Option<&str>,
    path: Option<&str>,
    filters: &[String],
) -> Option<FileAction> {
    setting_label(ui, label, tooltip);
    // Right to left, so the buttons keep their size and the path takes the
    // rest.
    // As tall as the field: centred in the whole height left, the row
    // would push everything after it down.
    let row = vec2(ui.available_width(), FIELD_HEIGHT);
    let action = ui
        .allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let browse = ui.add(Button::secondary("Browse…")).clicked();
            let clear = path.is_some() && clear_button(ui).clicked();
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
            if browse {
                Some(FileAction::Browse)
            } else {
                clear.then_some(FileAction::Clear)
            }
        })
        .inner;
    if !filters.is_empty() {
        ui.add_space(4.0);
        ui.add(
            Label::new(
                RichText::new(filters.join(" · "))
                    .font(theme::mono(11.0))
                    .color(theme::TEXT_MUTED),
            )
            .wrap(),
        );
    }
    action
}

/// The square secondary button with a ✕, drawn as two strokes rather than a
/// font glyph, that clears a file selection.
fn clear_button(ui: &mut Ui) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(FIELD_HEIGHT, FIELD_HEIGHT), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, "Clear"));
    if ui.is_rect_visible(rect) {
        let fill = if response.hovered() {
            theme::RAISED
        } else {
            Color32::TRANSPARENT
        };
        let painter = ui.painter();
        painter.rect(
            rect,
            theme::RADIUS,
            fill,
            Stroke::new(1.0, theme::BORDER_STRONG),
            StrokeKind::Inside,
        );
        if response.has_focus() {
            painter.rect_stroke(
                rect.expand(2.0),
                theme::RADIUS + 2,
                Stroke::new(1.0, theme::ACCENT),
                StrokeKind::Outside,
            );
        }
        let arm = 4.0;
        let stroke = Stroke::new(1.5, theme::TEXT_SECONDARY);
        let center = rect.center();
        painter.line_segment([center + vec2(-arm, -arm), center + vec2(arm, arm)], stroke);
        painter.line_segment([center + vec2(-arm, arm), center + vec2(arm, -arm)], stroke);
    }
    response.on_hover_text("Clear")
}
