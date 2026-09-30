//! The Splitter settings tab (spec §6.4): a fixed toolbar, and below it the
//! loaded auto splitter's settings, which scroll, followed in developer mode
//! by its whole settings map.

use std::time::Instant;

use eframe::egui::{Label, RichText, ScrollArea, Ui};
use livesplit_auto_splitting::{settings::WidgetKind, wasi_path};
use toml::Value;

use super::{
    components::{self, FileAction, SaveStatus, TextAction, ToolbarAction},
    settings::{Setting, SettingsEditor, filter_names},
    theme,
};

/// How far each heading level indents the settings under it.
const INDENT: f32 = 16.0;
/// The deepest indent, in levels, so deep trees don't squeeze the fields.
const MAX_INDENT_LEVELS: u32 = 3;
/// The height of a checkbox's row.
const CHECKBOX_ROW: f32 = 24.0;

/// What the Splitter settings tab asks the app to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    /// Save the draft.
    Save,
    /// Open a `.wasm` auto splitter.
    Open,
    /// Pick a file for the file selection with this key.
    Browse(String),
}

/// The Splitter settings tab. `loaded` is whether an auto splitter is
/// loaded. In developer mode, `settings_map` is whether the settings map is
/// shown, and the section to show or hide it follows the settings. Revert to
/// defaults and edits change the edits directly.
pub fn settings_tab(
    ui: &mut Ui,
    editor: &mut SettingsEditor,
    loaded: bool,
    settings_map: Option<&mut bool>,
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

    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        if editor.widgets().is_empty() {
            ui.add(
                Label::new(
                    RichText::new("This auto splitter has no settings.")
                        .font(theme::body(14.0))
                        .color(theme::TEXT_SECONDARY),
                )
                .wrap(),
            );
        }
        let widgets = editor.widgets().to_vec();
        let mut indent = 0.0;
        for (index, widget) in widgets.iter().enumerate() {
            if let WidgetKind::Title { heading_level } = widget.kind {
                indent = INDENT * heading_level.min(MAX_INDENT_LEVELS) as f32;
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
        if let Some(expanded) = settings_map {
            ui.add_space(24.0);
            let rect = ui.available_rect_before_wrap();
            ui.painter().hline(
                rect.x_range(),
                rect.top(),
                eframe::egui::Stroke::new(1.0, theme::BORDER),
            );
            ui.add_space(16.0);
            components::settings_map(ui, editor.live(), expanded, |key| {
                editor.recently_changed(key, now)
            });
            if editor
                .live()
                .keys()
                .any(|key| editor.recently_changed(key, now))
            {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(250));
            }
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
            // Rows as tall as a small button, so checkboxes don't crowd.
            let changed = ui
                .horizontal(|ui| {
                    ui.set_min_height(CHECKBOX_ROW);
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let response =
                        ui.add(components::Checkbox::new(&mut checked, &widget.description));
                    if let Some(tooltip) = tooltip {
                        components::info_marker(ui, tooltip);
                    }
                    response.changed()
                })
                .inner;
            if changed {
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
        WidgetKind::TextInput { default_value } => {
            ui.add_space(4.0);
            let mut text = value
                .as_ref()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            match components::setting_text(
                ui,
                &widget.description,
                tooltip,
                &mut text,
                default_value,
            ) {
                Some(TextAction::Changed) => editor.set(widget, Some(Value::String(text))),
                Some(TextAction::UseDefault) => {
                    editor.set(widget, Some(Value::String(default_value.to_string())));
                }
                None => {}
            }
            ui.add_space(12.0);
        }
        WidgetKind::FileSelect { filters } => {
            ui.add_space(4.0);
            let path = value.as_ref().and_then(Value::as_str).map(shown_path);
            let action = components::setting_file(
                ui,
                &widget.description,
                tooltip,
                path.as_deref(),
                &filter_names(filters),
            );
            ui.add_space(12.0);
            match action {
                Some(FileAction::Browse) => {
                    return Some(SettingsAction::Browse(widget.key.to_string()));
                }
                Some(FileAction::Clear) => editor.set(widget, None),
                None => {}
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
    use livesplit_auto_splitting::settings::{ChoiceOption, FileFilter};
    use toml::Table;

    use super::*;

    /// A widget on screen, as the accessibility tree describes it.
    #[derive(Debug)]
    struct Node {
        label: String,
        rect: Rect,
        disabled: bool,
    }

    /// The Splitter settings tab drawn without a window, with the app's fonts.
    struct Screen {
        ctx: Context,
        /// In developer mode, whether the settings map is shown.
        settings_map: std::cell::Cell<Option<bool>>,
    }

    impl Screen {
        fn new() -> Self {
            let ctx = Context::default();
            crate::ui::theme::install(&ctx);
            ctx.enable_accesskit();
            Self {
                ctx,
                settings_map: std::cell::Cell::new(None),
            }
        }

        /// The same, in developer mode.
        fn developer() -> Self {
            let screen = Self::new();
            screen.settings_map.set(Some(true));
            screen
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
            let mut settings_map = self.settings_map.get();
            let mut output = self.ctx.run_ui(input, |ui| {
                action = action.take().or(settings_tab(
                    ui,
                    editor,
                    loaded,
                    settings_map.as_mut(),
                    Instant::now(),
                ));
            });
            self.settings_map.set(settings_map);
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
                    filters: Arc::new(vec![
                        FileFilter::Name {
                            description: Some("Route files".into()),
                            pattern: "*.route".into(),
                        },
                        FileFilter::MimeType("image/*".into()),
                    ]),
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

    /// Types `text` into the focused widget.
    fn type_text(screen: &Screen, editor: &mut SettingsEditor, text: &str) {
        screen.frame(editor, true, vec![Event::Text(text.to_owned())]);
    }

    #[test]
    fn a_text_setting_shows_its_default_only_once_changed() {
        let screen = Screen::new();
        let mut editor = editor();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(!has(&nodes, "Default: me"), "{nodes:#?}");
        assert!(!has(&nodes, "Use default"));

        screen.click(&mut editor, true, "me");
        type_text(&screen, &mut editor, "!");
        assert_eq!(editor.draft()["runner"].as_str(), Some("me!"));
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(has(&nodes, "Default: me"), "{nodes:#?}");
        assert!(has(&nodes, "● Unsaved changes"));

        screen.click(&mut editor, true, "Use default");
        assert!(!editor.is_unsaved());
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(!has(&nodes, "Use default"), "{nodes:#?}");
    }

    #[test]
    fn an_empty_text_is_a_value_of_its_own() {
        let screen = Screen::new();
        let mut editor = editor();
        editor.set_live("runner = \"\"\n".parse().unwrap(), Instant::now());
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        // The field is empty, not showing the default.
        assert!(has(&nodes, "Default: me"), "{nodes:#?}");
        assert!(!has(&nodes, "me"), "{nodes:#?}");
    }

    #[test]
    fn a_file_setting_lists_its_filters_and_clears_its_file() {
        let screen = Screen::new();
        let mut editor = editor();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(has(&nodes, "Route files · Images"), "{nodes:#?}");
        assert!(has(&nodes, "No file selected"));
        // Nothing to clear yet.
        assert!(!has(&nodes, "Clear"));

        editor.set_live(
            "route = \"/mnt/c/r.route\"\n".parse().unwrap(),
            Instant::now(),
        );
        assert_eq!(screen.click(&mut editor, true, "Clear"), None);
        assert_eq!(editor.edits(), [("route".to_owned(), None)]);
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(has(&nodes, "No file selected"), "{nodes:#?}");
        assert!(has(&nodes, "● Unsaved changes"));
    }

    #[test]
    fn deep_headings_stop_indenting_after_three_levels() {
        let screen = Screen::new();
        let mut editor = SettingsEditor::default();
        editor.reset(Table::new());
        let mut widgets = Vec::new();
        for level in 0..6 {
            widgets.push(setting(
                &format!("h{level}"),
                &format!("Level {level}"),
                WidgetKind::Title {
                    heading_level: level,
                },
            ));
            widgets.push(setting(
                &format!("b{level}"),
                &format!("Box {level}"),
                WidgetKind::Bool {
                    default_value: false,
                },
            ));
        }
        editor.set_widgets(widgets);
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        let left = |label: &str| find(&nodes, label).rect.left();
        assert!(left("Box 1") > left("Box 0"));
        assert!(left("Box 2") > left("Box 1"));
        assert!(left("Box 3") > left("Box 2"));
        assert_eq!(left("Box 4"), left("Box 3"));
        assert_eq!(left("Box 5"), left("Box 3"));
    }

    #[test]
    fn the_settings_map_shows_only_in_developer_mode() {
        let mut editor = editor();
        editor.set_live("stored = 7\ngym = true\n".parse().unwrap(), Instant::now());
        let (_, nodes) = Screen::new().frame(&mut editor, true, Vec::new());
        assert!(!has(&nodes, "SETTINGS MAP · 2 VALUES"), "{nodes:#?}");
        assert!(!has(&nodes, "stored"));

        let screen = Screen::developer();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        for label in ["SETTINGS MAP · 2 VALUES", "DEVELOPER", "stored", "int", "7"] {
            assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
        }
        // Just stored by the auto splitter.
        assert!(has(&nodes, "changed"), "{nodes:#?}");

        screen.click(&mut editor, true, "Hide");
        assert_eq!(screen.settings_map.get(), Some(false));
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(!has(&nodes, "stored"), "{nodes:#?}");
        assert!(has(&nodes, "Show"));
    }

    #[test]
    fn lists_and_maps_in_the_settings_map_fold_open() {
        let mut editor = editor();
        editor.reset_keeping_widgets(
            "missions = [\"intro\", true]\n[last_run]\nsplits = 42\n"
                .parse()
                .unwrap(),
        );
        let screen = Screen::developer();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        assert!(has(&nodes, "2 items"), "{nodes:#?}");
        assert!(has(&nodes, "1 value"), "{nodes:#?}");
        assert!(!has(&nodes, "\"intro\""));

        screen.click(&mut editor, true, "missions");
        screen.click(&mut editor, true, "last_run");
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        for label in ["\"intro\"", "true", "splits", "42"] {
            assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
        }
        // Nothing in it can be edited.
        assert!(!editor.is_unsaved());
    }

    #[test]
    fn a_developer_mode_with_no_settings_still_shows_the_map() {
        let mut editor = SettingsEditor::default();
        editor.reset("stored = 1\n".parse().unwrap());
        let (_, nodes) = Screen::developer().frame(&mut editor, true, Vec::new());
        assert!(has(&nodes, "This auto splitter has no settings."));
        assert!(has(&nodes, "SETTINGS MAP · 1 VALUE"), "{nodes:#?}");
    }

    #[test]
    fn mockup_notes_are_not_shown() {
        let mut editor = editor();
        let (_, nodes) = Screen::developer().frame(&mut editor, true, Vec::new());
        assert!(
            !nodes
                .iter()
                .any(|node| node.label == "NEW" || node.label.contains("RECOMMENDED")),
            "{nodes:#?}"
        );
    }

    #[test]
    fn a_file_setting_takes_only_its_own_height() {
        let screen = Screen::new();
        let mut editor = editor();
        let (_, nodes) = screen.frame(&mut editor, true, Vec::new());
        let gap = find(&nodes, "Runner").rect.top() - find(&nodes, "Browse…").rect.bottom();
        assert!(gap < 60.0, "{gap} between the file and the next setting");
    }
}
