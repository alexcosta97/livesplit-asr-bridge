//! Server section: the port and Restart server, in the Connection tab.

use eframe::egui::{
    Align, Frame, Key, Label, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit,
    Ui, UiBuilder, vec2,
};

use super::{button::Button, section_label::section_label};
use crate::ui::theme;

/// The port field's size.
const FIELD_WIDTH: f32 = 120.0;
const FIELD_HEIGHT: f32 = 32.0;
/// The padding either side of the port in its field.
const FIELD_PADDING: f32 = 12.0;

/// What the port field holds, compared with the port the server was last
/// started with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortNote {
    /// The port the server was started with.
    Applied,
    /// Another valid port, which a restart applies.
    Edited,
    /// Not a port. A restart keeps the port the server was started with.
    Invalid,
}

/// The Server section: the port field and Restart server, always enabled,
/// with a note when the port was edited or isn't valid, and `error` below
/// when the server couldn't listen. Returns whether a restart was asked for,
/// with the button or Enter in the field.
pub fn server_section(
    ui: &mut Ui,
    port_text: &mut String,
    note: PortNote,
    error: Option<&str>,
) -> bool {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 16.0;
        section_label(ui, "Server");
        let (text, color) = match note {
            PortNote::Applied => ("", theme::TEXT),
            PortNote::Edited => ("● Restart the server to apply", theme::STATUS_WARNING),
            PortNote::Invalid => ("Enter a port from 1 to 65535", theme::STATUS_ERROR),
        };
        let note_text = RichText::new(text).font(theme::body(13.0)).color(color);
        let row = vec2(ui.available_width(), FIELD_HEIGHT);
        let (restart, note_fits) = ui
            .allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 8.0);
                ui.label(
                    RichText::new("Port")
                        .font(theme::body(14.0))
                        .color(theme::TEXT),
                );
                let entered = port_field(ui, port_text, note, error.is_some());
                // Primary while the server is stopped, as the way out.
                let button = if error.is_some() {
                    Button::primary("Restart server")
                } else {
                    Button::secondary("Restart server")
                };
                let clicked = ui
                    .add(button)
                    .on_hover_text("Close every connection and listen again")
                    .clicked();
                // The note goes after the button when it fits, else below.
                let note_width = ui
                    .painter()
                    .layout_no_wrap(text.to_owned(), theme::body(13.0), color)
                    .size()
                    .x;
                let note_fits = note_width <= ui.available_width();
                if note_fits && !text.is_empty() {
                    ui.label(note_text.clone());
                }
                (clicked || entered, note_fits)
            })
            .inner;
        if !note_fits && !text.is_empty() {
            ui.add(Label::new(note_text).wrap());
        }
        if let Some(error) = error {
            Frame::NONE
                .fill(theme::TINT_ERROR)
                .stroke(Stroke::new(1.0, theme::STATUS_ERROR))
                .corner_radius(theme::RADIUS)
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.add(
                        Label::new(
                            RichText::new(error)
                                .font(theme::body(13.0))
                                .color(theme::TEXT),
                        )
                        .wrap(),
                    );
                });
        }
        restart
    })
    .inner
}

/// The port field: mono text in a 120 px field, its border orange when
/// edited or focused and red when the port isn't valid or couldn't be used.
/// Returns whether Enter was pressed in it.
fn port_field(ui: &mut Ui, text: &mut String, note: PortNote, failed: bool) -> bool {
    let (rect, _) = ui.allocate_exact_size(vec2(FIELD_WIDTH, FIELD_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, theme::RADIUS, theme::PANEL);
    // One line of text, centred in the field, in a child that takes no
    // space of its own: the field already took it.
    let font = theme::mono(13.0);
    let line_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
    let inner = Rect::from_center_size(
        rect.center(),
        vec2(FIELD_WIDTH - 2.0 * FIELD_PADDING, line_height),
    );
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
    let response = child.add(
        TextEdit::singleline(text)
            .frame(Frame::NONE)
            .margin(Margin::ZERO)
            .font(font)
            .text_color(theme::TEXT)
            .char_limit(5)
            .desired_width(inner.width()),
    );
    let border = if note == PortNote::Invalid || (failed && note == PortNote::Applied) {
        theme::STATUS_ERROR
    } else if note == PortNote::Edited || response.has_focus() {
        theme::ACCENT
    } else {
        theme::BORDER_STRONG
    };
    ui.painter().rect_stroke(
        rect,
        theme::RADIUS,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter))
}
