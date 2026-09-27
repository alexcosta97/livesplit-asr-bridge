//! The Settings tab's state: the loaded auto splitter's widgets, its game's
//! saved settings, and the draft being edited (spec §6.4).
//!
//! Edits change only the draft. The running auto splitter keeps the saved
//! settings until they are saved, so a stray click mid-run can't change its
//! behaviour.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use livesplit_auto_splitting::settings::{Widget, WidgetKind};

pub use livesplit_auto_splitting::settings::{ChoiceOption, FileFilter};
use toml::{Table, Value};

use crate::config::{SettingKey, revert_to_defaults};

/// How long "✓ Saved" is shown after saving.
const SAVED_FOR: Duration = Duration::from_secs(3);

/// A widget the auto splitter publishes: a heading or a setting.
#[derive(Clone)]
pub struct Setting {
    pub key: Arc<str>,
    pub description: Arc<str>,
    pub tooltip: Option<Arc<str>>,
    pub kind: WidgetKind,
}

impl From<&Widget> for Setting {
    fn from(widget: &Widget) -> Self {
        Self {
            key: widget.key.clone(),
            description: widget.description.clone(),
            tooltip: widget.tooltip.clone(),
            kind: widget.kind.clone(),
        }
    }
}

/// The settings being edited.
#[derive(Default)]
pub struct SettingsEditor {
    widgets: Vec<Setting>,
    keys: Vec<SettingKey>,
    /// The game's saved settings, which the running auto splitter uses.
    saved: Table,
    /// The saved settings with the edits not saved yet.
    draft: Table,
    /// When the settings were last saved, for "✓ Saved".
    saved_at: Option<Instant>,
}

impl SettingsEditor {
    /// Starts again from `saved`, a game's saved settings, with no edits and
    /// no widgets until the auto splitter publishes them.
    pub fn reset(&mut self, saved: Table) {
        *self = Self {
            draft: saved.clone(),
            saved,
            ..Self::default()
        };
    }

    /// Starts again from `saved`, keeping the widgets, for example after the
    /// loaded auto splitter's game changed.
    pub fn reset_keeping_widgets(&mut self, saved: Table) {
        let widgets = std::mem::take(&mut self.widgets);
        self.reset(saved);
        self.set_widgets(widgets);
    }

    /// Shows the widgets the auto splitter published.
    pub fn set_widgets(&mut self, widgets: Vec<Setting>) {
        self.keys = widgets
            .iter()
            .filter_map(|widget| SettingKey::new(&widget.key, &widget.kind))
            .collect();
        self.widgets = widgets;
    }

    pub fn widgets(&self) -> &[Setting] {
        &self.widgets
    }

    /// The loaded auto splitter's settings keys.
    pub fn keys(&self) -> &[SettingKey] {
        &self.keys
    }

    /// The draft, to save.
    pub fn draft(&self) -> &Table {
        &self.draft
    }

    /// How many of the loaded auto splitter's settings the draft changes.
    pub fn changes(&self) -> usize {
        self.widgets
            .iter()
            .filter(|widget| {
                !matches!(widget.kind, WidgetKind::Title { .. })
                    && value(widget, &self.draft) != value(widget, &self.saved)
            })
            .count()
    }

    pub fn is_unsaved(&self) -> bool {
        self.changes() > 0
    }

    /// Replaces the draft's values with the auto splitter's defaults, as
    /// edits still to save. Other keys are unchanged.
    pub fn revert_to_defaults(&mut self) {
        self.draft = revert_to_defaults(&self.draft, &self.keys);
        self.saved_at = None;
    }

    /// Drops the edits.
    pub fn discard(&mut self) {
        self.draft = self.saved.clone();
    }

    /// Records that the settings were saved as `saved`.
    pub fn saved(&mut self, saved: Table, now: Instant) {
        self.draft = saved.clone();
        self.saved = saved;
        self.saved_at = Some(now);
    }

    /// Whether to show "✓ Saved" at `now`, and for how much longer.
    pub fn saved_recently(&self, now: Instant) -> Option<Duration> {
        let shown_for = now.saturating_duration_since(self.saved_at?);
        SAVED_FOR
            .checked_sub(shown_for)
            .filter(|left| !left.is_zero() && !self.is_unsaved())
    }

    /// The value `widget` shows from the draft.
    pub fn value(&self, widget: &Setting) -> Option<Value> {
        value(widget, &self.draft)
    }

    /// Sets `widget`'s value in the draft, or removes it with `None`.
    pub fn set(&mut self, widget: &Setting, value: Option<Value>) {
        match value {
            Some(value) => self.draft.insert(widget.key.to_string(), value),
            None => self.draft.remove(&*widget.key),
        };
        self.saved_at = None;
    }
}

/// The file dialog filters of a file selection: each name filter's
/// description and file extensions. Patterns other than `*.ext`, and MIME
/// type filters, can't be given to every OS's file dialog, so they are left
/// out.
pub fn file_filters(filters: &[FileFilter]) -> Vec<(String, Vec<String>)> {
    filters
        .iter()
        .filter_map(|filter| {
            let FileFilter::Name {
                description,
                pattern,
            } = filter
            else {
                return None;
            };
            let extensions: Vec<String> = pattern
                .split(' ')
                .filter_map(|pattern| pattern.strip_prefix("*."))
                .filter(|extension| {
                    !extension.is_empty() && !extension.contains(['*', '?', '[', '.'])
                })
                .map(str::to_owned)
                .collect();
            let name = description.as_deref().unwrap_or(pattern).to_owned();
            (!extensions.is_empty()).then_some((name, extensions))
        })
        .collect()
}

/// The value `widget` has in `settings`: the saved value if it is one the
/// widget can show, otherwise its default, as the runtime does. A file
/// selection has no default.
fn value(widget: &Setting, settings: &Table) -> Option<Value> {
    let saved = settings.get(&*widget.key);
    match &widget.kind {
        WidgetKind::Title { .. } => None,
        WidgetKind::Bool { default_value } => Some(Value::Boolean(
            saved.and_then(Value::as_bool).unwrap_or(*default_value),
        )),
        WidgetKind::Choice {
            default_option_key,
            options,
        } => {
            let saved = saved
                .and_then(Value::as_str)
                .filter(|key| options.iter().any(|option| &*option.key == *key));
            Some(Value::String(
                saved.unwrap_or(default_option_key).to_owned(),
            ))
        }
        WidgetKind::TextInput { default_value } => Some(Value::String(
            saved
                .and_then(Value::as_str)
                .unwrap_or(default_value)
                .to_owned(),
        )),
        WidgetKind::FileSelect { .. } => saved.filter(|value| value.is_str()).cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn widget(key: &str, kind: WidgetKind) -> Setting {
        Setting {
            key: key.into(),
            description: key.into(),
            tooltip: None,
            kind,
        }
    }

    fn widgets() -> Vec<Setting> {
        let option = |key: &str| ChoiceOption {
            key: key.into(),
            description: key.into(),
        };
        vec![
            widget("splits", WidgetKind::Title { heading_level: 0 }),
            widget(
                "gym",
                WidgetKind::Bool {
                    default_value: false,
                },
            ),
            widget(
                "category",
                WidgetKind::Choice {
                    default_option_key: "any".into(),
                    options: Arc::new(vec![option("any"), option("hundred")]),
                },
            ),
            widget(
                "route",
                WidgetKind::FileSelect {
                    filters: Arc::new(Vec::new()),
                },
            ),
        ]
    }

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn editor(saved: &str) -> SettingsEditor {
        let mut editor = SettingsEditor::default();
        editor.reset(table(saved));
        editor.set_widgets(widgets());
        editor
    }

    fn find(editor: &SettingsEditor, key: &str) -> Setting {
        editor
            .widgets()
            .iter()
            .find(|widget| &*widget.key == key)
            .unwrap()
            .clone()
    }

    #[test]
    fn widgets_show_saved_values_or_their_defaults() {
        let editor = editor("gym = true\ncategory = \"unknown\"\n");
        assert_eq!(
            editor.value(&find(&editor, "gym")),
            Some(Value::Boolean(true))
        );
        // A saved option the auto splitter doesn't offer shows the default.
        assert_eq!(
            editor.value(&find(&editor, "category")),
            Some(Value::String("any".to_owned()))
        );
        assert_eq!(editor.value(&find(&editor, "route")), None);
        assert_eq!(editor.value(&find(&editor, "splits")), None);
    }

    #[test]
    fn edits_are_unsaved_until_saved() {
        let mut editor = editor("other = 1\n");
        assert!(!editor.is_unsaved());
        let gym = find(&editor, "gym");
        editor.set(&gym, Some(Value::Boolean(true)));
        editor.set(
            &find(&editor, "route"),
            Some(Value::String("/mnt/c/route.txt".to_owned())),
        );
        assert_eq!(editor.changes(), 2);

        let saved = editor.draft().clone();
        editor.saved(saved, Instant::now());
        assert!(!editor.is_unsaved());
        assert_eq!(editor.value(&gym), Some(Value::Boolean(true)));
    }

    #[test]
    fn setting_a_value_back_leaves_nothing_to_save() {
        let mut editor = editor("");
        let gym = find(&editor, "gym");
        editor.set(&gym, Some(Value::Boolean(true)));
        editor.set(&gym, Some(Value::Boolean(false)));
        assert!(!editor.is_unsaved());
    }

    #[test]
    fn discarding_drops_the_edits() {
        let mut editor = editor("gym = true\n");
        editor.set(&find(&editor, "gym"), Some(Value::Boolean(false)));
        editor.discard();
        assert!(!editor.is_unsaved());
        assert_eq!(editor.draft(), &table("gym = true\n"));
    }

    #[test]
    fn reverting_to_defaults_is_unsaved_until_saved() {
        let mut editor =
            editor("gym = true\ncategory = \"hundred\"\nroute = \"/mnt/c/r.txt\"\nother = 1\n");
        editor.revert_to_defaults();
        assert_eq!(editor.changes(), 3);
        assert_eq!(
            editor.draft(),
            &table("gym = false\ncategory = \"any\"\nother = 1\n")
        );
    }

    #[test]
    fn reverting_settings_already_at_their_defaults_changes_nothing() {
        let mut editor = editor("gym = false\n");
        editor.revert_to_defaults();
        assert!(!editor.is_unsaved());
    }

    #[test]
    fn saved_shows_briefly_and_not_once_edited_again() {
        let mut editor = editor("");
        let start = Instant::now();
        editor.saved(Table::new(), start);
        assert!(editor.saved_recently(start).is_some());
        assert!(editor.saved_recently(start + SAVED_FOR).is_none());

        editor.set(&find(&editor, "gym"), Some(Value::Boolean(true)));
        assert!(editor.saved_recently(start).is_none());
    }

    #[test]
    fn file_filters_keep_extension_patterns() {
        let filters = [
            FileFilter::Name {
                description: Some("Route files".into()),
                pattern: "*.txt *.route".into(),
            },
            FileFilter::Name {
                description: None,
                pattern: "*.csv".into(),
            },
            FileFilter::Name {
                description: Some("Anything".into()),
                pattern: "route_* *".into(),
            },
            FileFilter::MimeType("image/*".into()),
        ];
        assert_eq!(
            file_filters(&filters),
            [
                (
                    "Route files".to_owned(),
                    vec!["txt".to_owned(), "route".to_owned()]
                ),
                ("*.csv".to_owned(), vec!["csv".to_owned()]),
            ]
        );
    }

    #[test]
    fn widgets_arrive_after_a_reset_and_stay_when_the_game_changes() {
        let mut editor = editor("");
        editor.reset_keeping_widgets(table("gym = true\n"));
        assert_eq!(editor.widgets().len(), 4);
        assert!(!editor.is_unsaved());
        editor.reset(Table::new());
        assert!(editor.widgets().is_empty());
        assert!(editor.keys().is_empty());
    }
}
