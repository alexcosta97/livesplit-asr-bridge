//! About row: a folder in the Preferences tab's About section, its name
//! above its path, with **Open** on the right.

use std::path::Path;

use eframe::egui::{Align, Label, Layout, RichText, Ui, vec2};

use super::button::Button;
use crate::ui::theme;

/// The widest the row gets, with **Open** at its right end.
const MAX_WIDTH: f32 = 600.0;
/// The space between the path and **Open**.
const GAP: f32 = 16.0;
/// The space between the name and the path.
const NAME_GAP: f32 = 2.0;
/// The height of **Open**, the row's least height.
const BUTTON_HEIGHT: f32 = 32.0;

/// A row naming a folder, with its path and **Open**. Returns whether Open
/// was clicked. With no folder, the row says it wasn't found and Open is
/// disabled.
pub fn about_folder(ui: &mut Ui, name: &str, dir: Option<&Path>) -> bool {
    let name_font = theme::body(13.0);
    let path_font = theme::mono(13.0);
    let text_height = ui
        .fonts_mut(|fonts| fonts.row_height(&name_font) + fonts.row_height(&path_font))
        + NAME_GAP;
    let height = text_height.max(BUTTON_HEIGHT);
    let width = ui.available_width().min(MAX_WIDTH);
    let mut clicked = false;
    ui.allocate_ui_with_layout(
        vec2(width, height),
        Layout::right_to_left(Align::Center),
        |ui| {
            ui.set_min_size(vec2(width, height));
            clicked = ui
                .add(Button::secondary("Open").enabled(dir.is_some()))
                .clicked();
            ui.add_space(GAP);
            // The name and path take the rest of the row, from the left.
            ui.allocate_ui_with_layout(
                vec2(ui.available_width(), text_height),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = NAME_GAP;
                    ui.label(
                        RichText::new(name)
                            .font(name_font)
                            .color(theme::TEXT_SECONDARY),
                    );
                    match dir {
                        Some(dir) => {
                            let path = dir.display().to_string();
                            ui.add(
                                Label::new(RichText::new(&path).font(path_font).color(theme::TEXT))
                                    .truncate(),
                            )
                            .on_hover_text(path);
                        }
                        None => {
                            ui.label(
                                RichText::new("Not found")
                                    .font(path_font)
                                    .color(theme::TEXT_MUTED),
                            );
                        }
                    }
                },
            );
        },
    );
    clicked
}
