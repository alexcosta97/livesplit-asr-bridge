//! The Settings tab (spec §6.4): a fixed toolbar, and below it the loaded
//! auto splitter's settings, which scroll.

use std::time::Instant;

use eframe::egui::{Label, RichText, ScrollArea, Ui};
use livesplit_auto_splitting::{settings::WidgetKind, wasi_path};
use toml::Value;

use super::{
    components::{self, SaveStatus, ToolbarAction},
    settings::{Setting, SettingsEditor},
    theme,
};

/// How far each heading level indents the settings under it.
const INDENT: f32 = 16.0;

/// What the Settings tab asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    /// Save the draft.
    Save,
    /// Open a `.wasm` auto splitter.
    Open,
    /// Pick a file for the file selection with this key.
    Browse(String),
}

/// The Settings tab. `loaded` is whether an auto splitter is loaded.
/// Revert to defaults and edits change the draft directly.
pub fn settings_tab(
    ui: &mut Ui,
    editor: &mut SettingsEditor,
    loaded: bool,
    now: Instant,
) -> Option<SettingsAction> {
    if !loaded {
        return components::empty_state(ui).then_some(SettingsAction::Open);
    }

    let status = if editor.is_unsaved() {
        SaveStatus::Unsaved
    } else if let Some(left) = editor.saved_recently(now) {
        ui.ctx().request_repaint_after(left);
        SaveStatus::Saved
    } else {
        SaveStatus::Nothing
    };
    let mut action = match components::settings_toolbar(ui, status) {
        Some(ToolbarAction::Save) => Some(SettingsAction::Save),
        Some(ToolbarAction::RevertToDefaults) => {
            editor.revert_to_defaults();
            None
        }
        None => None,
    };
    ui.add_space(16.0);

    if editor.widgets().is_empty() {
        ui.add(
            Label::new(
                RichText::new("This auto splitter has no settings.")
                    .font(theme::body(14.0))
                    .color(theme::TEXT_SECONDARY),
            )
            .wrap(),
        );
        return action;
    }

    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        let widgets = editor.widgets().to_vec();
        let mut indent = 0.0;
        for (index, widget) in widgets.iter().enumerate() {
            if let WidgetKind::Title { heading_level } = widget.kind {
                indent = INDENT * heading_level as f32;
                if index > 0 {
                    ui.add_space(if heading_level == 0 { 20.0 } else { 12.0 });
                }
            }
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(indent);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if let Some(browse) = setting(ui, editor, widget) {
                        action = Some(browse);
                    }
                });
            });
        }
        ui.add_space(16.0);
    });
    action
}

/// One widget: a heading, or a setting that edits the draft.
fn setting(ui: &mut Ui, editor: &mut SettingsEditor, widget: &Setting) -> Option<SettingsAction> {
    let tooltip = widget.tooltip.as_deref();
    let value = editor.value(widget);
    match &widget.kind {
        WidgetKind::Title { heading_level } => {
            components::setting_heading(ui, &widget.description, *heading_level, tooltip);
            ui.add_space(8.0);
        }
        WidgetKind::Bool { .. } => {
            let mut checked = value.and_then(|value| value.as_bool()).unwrap_or_default();
            let response =
                components::checkbox(ui, &mut checked, &widget.description, tooltip, None);
            if response.changed() {
                editor.set(widget, Some(Value::Boolean(checked)));
            }
            ui.add_space(4.0);
        }
        WidgetKind::Choice { options, .. } => {
            ui.add_space(4.0);
            let selected = value
                .as_ref()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Some(picked) = components::setting_choice(
                ui,
                &widget.key,
                &widget.description,
                tooltip,
                options,
                &selected,
            ) {
                editor.set(widget, Some(Value::String(picked)));
            }
            ui.add_space(12.0);
        }
        WidgetKind::TextInput { .. } => {
            ui.add_space(4.0);
            let mut text = value
                .as_ref()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let response = components::setting_text(ui, &widget.description, tooltip, &mut text);
            if response.changed() {
                editor.set(widget, Some(Value::String(text)));
            }
            ui.add_space(12.0);
        }
        WidgetKind::FileSelect { .. } => {
            ui.add_space(4.0);
            let path = value.as_ref().and_then(Value::as_str).map(shown_path);
            let browse =
                components::setting_file(ui, &widget.description, tooltip, path.as_deref());
            ui.add_space(12.0);
            if browse {
                return Some(SettingsAction::Browse(widget.key.to_string()));
            }
        }
    }
    None
}

/// A file selection's value, a path in the auto splitter's file system, as a
/// path on this machine.
fn shown_path(value: &str) -> String {
    wasi_path::to_native(value, false)
        .map_or_else(|| value.to_owned(), |path| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use eframe::egui::{
        Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, accesskit, pos2, vec2,
    };
    use livesplit_auto_splitting::settings::ChoiceOption;
    use toml::Table;

    use super::*;

    /// A widget on screen, as the accessibility tree describes it.
    #[derive(Debug)]
    struct Node {
        label: String,
        rect: Rect,
        disabled: bool,
    }

    /// The Settings tab drawn without a window, with the app's fonts.
    struct Screen {
        ctx: Context,
    }

    impl Screen {
        fn new() -> Self {
            let ctx = Context::default();
            crate::ui::theme::install(&ctx);
            ctx.enable_accesskit();
            Self { ctx }
        }

        /// Draws one frame with `events`, returning the tab's action and
        /// what is on screen.
        fn frame(
            &self,
            editor: &mut SettingsEditor,
            loaded: bool,
            events: Vec<Event>,
        ) -> (Option<SettingsAction>, Vec<Node>) {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 900.0))),
                events,
                ..RawInput::default()
            };
            let mut action = None;
            let mut output = self.ctx.run_ui(input, |ui| {
                action = action
                    .take()
                    .or(settings_tab(ui, editor, loaded, Instant::now()));
            });
            // Nothing draws the frame, so its textures are dropped.
            output.textures_delta.clear();
            let nodes = output
                .platform_output
                .accesskit_update
                .map(|update| update.nodes)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(_, node): (accesskit::NodeId, accesskit::Node)| {
                    let bounds = node.bounds()?;
                    Some(Node {
                        label: node.label().or(node.value())?.to_owned(),
                        rect: Rect::from_min_max(
                            pos2(bounds.x0 as f32, bounds.y0 as f32),
                            pos2(bounds.x1 as f32, bounds.y1 as f32),
                        ),
                        disabled: node.is_disabled(),
                    })
                })
                .collect();
            (action, nodes)
        }

        /// Clicks the widget labelled `label`, returning the tab's action.
        fn click(
            &self,
            editor: &mut SettingsEditor,
            loaded: bool,
            label: &str,
        ) -> Option<SettingsAction> {
            let (_, nodes) = self.frame(editor, loaded, Vec::new());
            let position = find(&nodes, label).rect.center();
            let button = |pressed| Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            let (pressed, _) = self.frame(
                editor,
                loaded,
                vec![Event::PointerMoved(position), button(true)],
            );
            let (released, _) = self.frame(editor, loaded, vec![button(false)]);
            pressed.or(released)
        }
    }

    fn find<'a>(nodes: &'a [Node], label: &str) -> &'a Node {
        nodes
            .iter()
            .find(|node| node.label == label)
            .unwrap_or_else(|| panic!("no {label:?} in {nodes:#?}"))
    }

    fn has(nodes: &[Node], label: &str) -> bool {
        nodes.iter().any(|node| node.label == label)
    }

    fn setting(key: &str, description: &str, kind: WidgetKind) -> Setting {
        Setting {
            key: key.into(),
            description: description.into(),
            tooltip: None,
            kind,
        }
    }

    /// One widget of every kind.
    fn editor() -> SettingsEditor {
        let mut editor = SettingsEditor::default();
        editor.reset(Table::new());
        editor.set_widgets(vec![
            setting("splits", "Splits", WidgetKind::Title { heading_level: 0 }),
            setting(
                "gym",
                "Gym Moves",
                WidgetKind::Bool {
                    default_value: false,
                },
            ),
            setting("extras", "Extras", WidgetKind::Title { heading_level: 1 }),
            setting(
                "category",
                "Category",
                WidgetKind::Choice {
                    default_option_key: "any".into(),
                    options: Arc::new(vec![ChoiceOption {
                        key: "any".into(),
                        description: "Any%".into(),
                    }]),
                },
            ),
            setting(
                "route",
                "Route",
                WidgetKind::FileSelect {
                    filters: Arc::new(Vec::new()),
                },
            ),
            setting(
                "runner",
                "Runner",
                WidgetKind::TextInput {
                    default_value: "me".into(),
                },
            ),
        ]);
        editor
    }

    #[test]
    fn shows_every_kind_of_widget_and_nothing_to_save() {
        let screen = Screen::new();
        let mut editor = editor();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        for label in [
            "SPLITS",
            "Gym Moves",
            "Extras",
            "Category",
            "Route",
            "Browse…",
            "Runner",
        ] {
            assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
        }
        // A level 2 heading indents the settings under it.
        assert!(find(&nodes, "Category").rect.left() > find(&nodes, "Gym Moves").rect.left());
        assert!(find(&nodes, "Save").disabled);
        assert!(!has(&nodes, "● Unsaved changes"));
    }

    #[test]
    fn an_edit_is_unsaved_until_save_is_pressed() {
        let screen = Screen::new();
        let mut editor = editor();
        assert_eq!(screen.click(&mut editor, true, "Gym Moves"), None);
        assert_eq!(editor.changes(), 1);
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(!find(&nodes, "Save").disabled);
        assert!(has(&nodes, "● Unsaved changes"));

        assert_eq!(
            screen.click(&mut editor, true, "Save"),
            Some(SettingsAction::Save)
        );
    }

    #[test]
    fn revert_to_defaults_fills_the_draft_as_unsaved_changes() {
        let screen = Screen::new();
        let mut editor = editor();
        editor.reset_keeping_widgets("gym = true\n".parse().unwrap());
        screen.click(&mut editor, true, "Revert to defaults");
        assert_eq!(editor.changes(), 1);
        assert_eq!(editor.draft()["gym"].as_bool(), Some(false));
    }

    #[test]
    fn browse_asks_for_the_file_of_its_setting() {
        let screen = Screen::new();
        let mut editor = editor();
        assert_eq!(
            screen.click(&mut editor, true, "Browse…"),
            Some(SettingsAction::Browse("route".to_owned()))
        );
    }

    #[test]
    fn with_nothing_loaded_explains_how_to_open_one() {
        let screen = Screen::new();
        let mut editor = SettingsEditor::default();
        let (_, nodes) = screen.frame(&mut editor, false, Vec::new());
        assert!(has(&nodes, "NO AUTO SPLITTER LOADED"), "{nodes:#?}");
        assert!(!has(&nodes, "Save"));
        assert_eq!(
            screen.click(&mut editor, false, "Open…"),
            Some(SettingsAction::Open)
        );
    }
}
