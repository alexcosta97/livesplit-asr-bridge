//! About row: a line of the Preferences tab's About section, like the
//! version, or a folder with **Open**.

use std::path::Path;

use eframe::egui::{
    Align, Frame, Label, Layout, Margin, Response, RichText, Stroke, Ui, UiBuilder, vec2,
};

use super::button::{Button, ButtonSize};
use crate::ui::theme;

/// The row's height.
const HEIGHT: f32 = 48.0;
/// The width of the name column.
const NAME_WIDTH: f32 = 120.0;

/// A row with a name and its value, like the version.
pub fn about_row(ui: &mut Ui, name: &str, value: &str) {
    row(ui, name, |ui| {
        value_label(ui, value);
    });
}

/// A row naming a folder, with its path and **Open**. Returns whether Open
/// was clicked. With no folder, the row says it wasn't found and Open is
/// disabled.
pub fn about_folder(ui: &mut Ui, name: &str, dir: Option<&Path>) -> bool {
    let mut clicked = false;
    row(ui, name, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            clicked = ui
                .add(
                    Button::secondary("Open")
                        .size(ButtonSize::Sm)
                        .enabled(dir.is_some()),
                )
                .clicked();
            ui.add_space(8.0);
            // The path takes the rest of the row, from the left.
            let rest = ui.available_rect_before_wrap();
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(rest)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| match dir {
                    Some(dir) => {
                        let path = dir.display().to_string();
                        value_label(ui, &path).on_hover_text(path);
                    }
                    None => {
                        ui.label(
                            RichText::new("Not found")
                                .font(theme::body(13.0))
                                .color(theme::TEXT_MUTED),
                        );
                    }
                },
            );
        });
    });
    clicked
}

/// The row's frame and name, then `value` in the rest of the row.
fn row(ui: &mut Ui, name: &str, value: impl FnOnce(&mut Ui)) {
    Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // The frame's 1 px border is outside the inner height.
            ui.set_height(HEIGHT - 2.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.allocate_ui_with_layout(
                    vec2(NAME_WIDTH, HEIGHT - 2.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_min_width(NAME_WIDTH);
                        ui.label(
                            RichText::new(name)
                                .font(theme::body(13.0))
                                .color(theme::TEXT_SECONDARY),
                        );
                    },
                );
                value(ui);
            });
        });
}

/// A value in mono text, cut short with "…" when it doesn't fit.
fn value_label(ui: &mut Ui, value: &str) -> Response {
    ui.add(
        Label::new(
            RichText::new(value)
                .font(theme::mono(12.0))
                .color(theme::TEXT),
        )
        .truncate(),
    )
}
