//! Settings map: the running auto splitter's whole settings map, read only,
//! shown under the settings in developer mode (spec §6.4). Lists and maps
//! fold open; a value the auto splitter just changed itself is tinted.

use eframe::egui::{
    Align, Color32, Frame, Id, Label, Layout, Link, Margin, Pos2, RichText, Sense, Shape, Stroke,
    Ui, Vec2, pos2, vec2,
};
use toml::{Table, Value};

use super::section_label::section_label;
use crate::ui::theme;

/// The width of the key column, at the top level.
const KEY_WIDTH: f32 = 220.0;
/// The width of the type column.
const TYPE_WIDTH: f32 = 64.0;
/// How far each level of a list or map indents its entries.
const INDENT: f32 = 16.0;
/// The height of a row.
const ROW_HEIGHT: f32 = 26.0;

/// The settings map section: its label with the number of values and a
/// DEVELOPER tag, Hide or Show at the right, then, when `expanded`, one row
/// per value. `changed` says whether the auto splitter changed a top-level
/// key itself just now.
pub fn settings_map(
    ui: &mut Ui,
    live: &Table,
    expanded: &mut bool,
    changed: impl Fn(&str) -> bool,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let count = match live.len() {
            1 => "1 value".to_owned(),
            n => format!("{n} values"),
        };
        section_label(ui, &format!("Settings map · {count}"));
        developer_tag(ui);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let text = if *expanded { "Hide" } else { "Show" };
            if ui
                .add(Link::new(
                    RichText::new(text).font(theme::body_semibold(12.0)),
                ))
                .clicked()
            {
                *expanded = !*expanded;
            }
        });
    });
    if !*expanded {
        return;
    }
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new(
                "What the running auto splitter sees right now, including values it stored \
                 itself. Read only; updates live.",
            )
            .font(theme::body(12.0))
            .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    ui.add_space(10.0);
    if live.is_empty() {
        ui.label(
            RichText::new("The settings map is empty.")
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
        );
        return;
    }
    header(ui);
    let base = ui.id().with("settings map");
    for (key, value) in live {
        row(ui, base, key, value, 0, changed(key));
    }
}

/// The small outlined DEVELOPER tag.
fn developer_tag(ui: &mut Ui) {
    Frame::NONE
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(2)
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.label(
                RichText::new("DEVELOPER")
                    .font(theme::mono(10.0))
                    .color(theme::TEXT_SECONDARY),
            );
        });
}

/// The column labels.
fn header(ui: &mut Ui) {
    let cell = |text: &str| {
        RichText::new(text)
            .font(theme::mono(10.0))
            .color(theme::TEXT_MUTED)
            .extra_letter_spacing(0.8)
    };
    columns(ui, 0, false, |ui, column| match column {
        0 => {
            ui.label(cell("KEY"));
        }
        1 => {
            ui.label(cell("TYPE"));
        }
        _ => {
            ui.label(cell("VALUE"));
        }
    });
}

/// One value's row, then its entries' rows if it is a list or map that is
/// open. Rows fold open by clicking their key.
fn row(ui: &mut Ui, parent: Id, key: &str, value: &Value, depth: usize, changed: bool) {
    let id = parent.with(key);
    let nested = matches!(value, Value::Array(_) | Value::Table(_));
    let mut open = nested && ui.data(|data| data.get_temp(id).unwrap_or(false));
    columns(ui, depth, changed, |ui, column| match column {
        0 => {
            if nested {
                let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                disclosure(ui, rect.center(), open);
            }
            let label = Label::new(
                RichText::new(key)
                    .font(theme::mono(12.0))
                    .color(theme::TEXT),
            )
            .truncate();
            let response = if nested {
                ui.add(label.sense(Sense::click()))
            } else {
                ui.add(label)
            };
            if nested && response.clicked() {
                open = !open;
                ui.data_mut(|data| data.insert_temp(id, open));
            }
        }
        1 => {
            ui.label(
                RichText::new(type_name(value))
                    .font(theme::mono(12.0))
                    .color(theme::TEXT_SECONDARY),
            );
        }
        _ => {
            let (text, color) = shown(value);
            ui.add(Label::new(RichText::new(text).font(theme::mono(12.0)).color(color)).truncate());
            if changed {
                ui.label(
                    RichText::new("changed")
                        .font(theme::mono(11.0))
                        .color(theme::ACCENT),
                );
            }
        }
    });
    if !open {
        return;
    }
    match value {
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                row(ui, id, &index.to_string(), value, depth + 1, false);
            }
        }
        Value::Table(table) => {
            for (key, value) in table {
                row(ui, id, key, value, depth + 1, false);
            }
        }
        _ => {}
    }
}

/// Lays out a row's three columns, indenting the key by `depth`, with a
/// 1 px line under it, tinted when `changed`.
fn columns(ui: &mut Ui, depth: usize, changed: bool, mut cell: impl FnMut(&mut Ui, usize)) {
    let fill = if changed {
        theme::TINT_ACCENT
    } else {
        Color32::TRANSPARENT
    };
    let response = Frame::NONE.fill(fill).show(ui, |ui| {
        let row = vec2(ui.available_width(), ROW_HEIGHT);
        ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
            ui.set_min_size(row);
            ui.spacing_mut().item_spacing.x = 6.0;
            let indent = INDENT * depth as f32;
            ui.add_space(indent);
            for (column, width) in [(0, KEY_WIDTH - indent), (1, TYPE_WIDTH)] {
                ui.allocate_ui_with_layout(
                    vec2(width.max(40.0), ROW_HEIGHT),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.set_width(width.max(40.0));
                        cell(ui, column);
                    },
                );
                ui.add_space(6.0);
            }
            cell(ui, 2);
        });
    });
    let rect = response.response.rect;
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, theme::BORDER),
    );
}

/// The ▸ or ▾ of a list or map, drawn as a triangle.
fn disclosure(ui: &Ui, center: Pos2, open: bool) {
    let points = if open {
        vec![
            center + Vec2::new(-4.0, -2.0),
            center + Vec2::new(4.0, -2.0),
            center + Vec2::new(0.0, 3.0),
        ]
    } else {
        vec![
            pos2(center.x - 2.0, center.y - 4.0),
            pos2(center.x - 2.0, center.y + 4.0),
            pos2(center.x + 3.0, center.y),
        ]
    };
    ui.painter().add(Shape::convex_polygon(
        points,
        theme::TEXT_SECONDARY,
        Stroke::NONE,
    ));
}

/// The name of a value's type.
fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Boolean(_) => "bool",
        Value::Integer(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Array(_) => "list",
        Value::Table(_) => "map",
        Value::Datetime(_) => "date",
    }
}

/// A value as shown in its row, and its colour: lists and maps by size.
fn shown(value: &Value) -> (String, Color32) {
    match value {
        Value::Boolean(value) => (value.to_string(), theme::TEXT),
        Value::Integer(value) => (value.to_string(), theme::TEXT),
        Value::Float(value) => (format!("{value:?}"), theme::TEXT),
        Value::String(value) => (format!("{value:?}"), theme::TEXT),
        Value::Datetime(value) => (value.to_string(), theme::TEXT),
        Value::Array(values) => (
            match values.len() {
                1 => "1 item".to_owned(),
                n => format!("{n} items"),
            },
            theme::TEXT_SECONDARY,
        ),
        Value::Table(table) => (
            match table.len() {
                1 => "1 value".to_owned(),
                n => format!("{n} values"),
            },
            theme::TEXT_SECONDARY,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_shown_by_type() {
        let table: Table = "a = true\nb = 7\nc = 1.0\nd = \"Any%\"\ne = [1, 2]\nf = { x = 1 }\n"
            .parse()
            .unwrap();
        let shown: Vec<_> = table
            .values()
            .map(|value| (type_name(value), shown(value).0))
            .collect();
        assert_eq!(
            shown,
            [
                ("bool", "true".to_owned()),
                ("int", "7".to_owned()),
                ("float", "1.0".to_owned()),
                ("string", "\"Any%\"".to_owned()),
                ("list", "2 items".to_owned()),
                ("map", "1 value".to_owned()),
            ]
        );
    }
}
