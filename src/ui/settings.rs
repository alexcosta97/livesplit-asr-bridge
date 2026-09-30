//! The Splitter settings tab's state: the loaded auto splitter's widgets, its
//! settings map as the runtime has it, and the edits not saved yet (spec
//! §6.4, §7.2).
//!
//! As in LiveSplit, the running auto splitter's settings map is the truth:
//! widgets show its values, and follow them when the auto splitter stores
//! values itself. Edits wait until they are saved, so a stray click mid-run
//! can't change the auto splitter's behaviour; an edit shows instead of the
//! map's value until then. Saving writes only the edited keys into the map.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

use livesplit_auto_splitting::settings::{Widget, WidgetKind};

pub use livesplit_auto_splitting::settings::{ChoiceOption, FileFilter};
use toml::{Table, Value};

use crate::config::{SettingKey, revert_to_defaults};

/// How long "✓ Saved" is shown after saving.
const SAVED_FOR: Duration = Duration::from_secs(3);
/// How long a value the auto splitter changed is marked as changed.
pub const CHANGED_FOR: Duration = Duration::from_secs(3);
/// The file dialog filter that shows every file, last, as in LiveSplit.
pub const ALL_FILES: &str = "All files (*.*)";

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
    /// The running auto splitter's settings map, as last reported.
    live: Table,
    /// The edits not saved yet: each key's new value, or `None` to remove
    /// it. Only edits that change what a widget shows are kept.
    edits: BTreeMap<String, Option<Value>>,
    /// When the settings were last saved, for "✓ Saved".
    saved_at: Option<Instant>,
    /// When the auto splitter last changed each key itself.
    changed_at: HashMap<String, Instant>,
}

impl SettingsEditor {
    /// Starts again from `live`, the settings map an auto splitter was given,
    /// with no edits and no widgets until the auto splitter publishes them.
    pub fn reset(&mut self, live: Table) {
        *self = Self {
            live,
            ..Self::default()
        };
    }

    /// Starts again from `live`, keeping the widgets, for example after the
    /// loaded auto splitter's game changed.
    pub fn reset_keeping_widgets(&mut self, live: Table) {
        let widgets = std::mem::take(&mut self.widgets);
        self.reset(live);
        self.set_widgets(widgets);
    }

    /// Shows the widgets the auto splitter published. Edits of settings it
    /// no longer has are dropped.
    pub fn set_widgets(&mut self, widgets: Vec<Setting>) {
        self.keys = widgets
            .iter()
            .filter_map(|widget| SettingKey::new(&widget.key, &widget.kind))
            .collect();
        self.widgets = widgets;
        self.prune();
    }

    pub fn widgets(&self) -> &[Setting] {
        &self.widgets
    }

    /// The loaded auto splitter's settings keys.
    #[cfg(test)]
    pub fn keys(&self) -> &[SettingKey] {
        &self.keys
    }

    /// The running auto splitter's settings map, as last reported.
    pub fn live(&self) -> &Table {
        &self.live
    }

    /// The settings map changed in the runtime: the auto splitter stored
    /// values, or saved edits reached it. Returns the keys whose values
    /// changed, which are marked as changed at `now`. Edits that no longer
    /// change anything are dropped; the others still show.
    pub fn set_live(&mut self, live: Table, now: Instant) -> Vec<String> {
        let mut changed: Vec<String> = live
            .iter()
            .filter(|(key, value)| self.live.get(*key) != Some(value))
            .map(|(key, _)| key.clone())
            .collect();
        changed.extend(
            self.live
                .keys()
                .filter(|key| !live.contains_key(*key))
                .cloned(),
        );
        for key in &changed {
            self.changed_at.insert(key.clone(), now);
        }
        self.live = live;
        self.prune();
        changed
    }

    /// Whether the auto splitter changed `key` itself within
    /// [`CHANGED_FOR`] of `now`.
    pub fn recently_changed(&self, key: &str, now: Instant) -> bool {
        self.changed_at
            .get(key)
            .is_some_and(|at| now.saturating_duration_since(*at) < CHANGED_FOR)
    }

    /// The settings map with the edits applied: what saving leaves in the
    /// runtime.
    pub fn draft(&self) -> Table {
        let mut draft = self.live.clone();
        for (key, value) in &self.edits {
            match value {
                Some(value) => draft.insert(key.clone(), value.clone()),
                None => draft.remove(key),
            };
        }
        draft
    }

    /// The edits to write into the runtime's map when saving.
    pub fn edits(&self) -> Vec<(String, Option<Value>)> {
        self.edits
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    /// How many of the loaded auto splitter's settings the edits change.
    pub fn changes(&self) -> usize {
        self.edits.len()
    }

    pub fn is_unsaved(&self) -> bool {
        self.changes() > 0
    }

    /// Sets the auto splitter's settings to their defaults, as edits still
    /// to save. Other keys are unchanged.
    pub fn revert_to_defaults(&mut self) {
        let reverted = revert_to_defaults(&self.draft(), &self.keys);
        for SettingKey { key, .. } in &self.keys {
            self.edits.insert(key.clone(), reverted.get(key).cloned());
        }
        self.saved_at = None;
        self.prune();
    }

    /// Drops the edits.
    pub fn discard(&mut self) {
        self.edits.clear();
    }

    /// Records that the edits were saved at `now`: the runtime's map now
    /// holds them.
    pub fn saved(&mut self, now: Instant) {
        self.live = self.draft();
        self.edits.clear();
        self.saved_at = Some(now);
    }

    /// Whether to show "✓ Saved" at `now`, and for how much longer.
    pub fn saved_recently(&self, now: Instant) -> Option<Duration> {
        let shown_for = now.saturating_duration_since(self.saved_at?);
        SAVED_FOR
            .checked_sub(shown_for)
            .filter(|left| !left.is_zero() && !self.is_unsaved())
    }

    /// The value `widget` shows: its edit, or else the runtime's value.
    pub fn value(&self, widget: &Setting) -> Option<Value> {
        match self.edits.get(&*widget.key) {
            Some(edit) => value(widget, edit.as_ref()),
            None => value(widget, self.live.get(&*widget.key)),
        }
    }

    /// Sets `widget`'s value as an edit, or removes it with `None`.
    pub fn set(&mut self, widget: &Setting, value: Option<Value>) {
        self.edits.insert(widget.key.to_string(), value);
        self.saved_at = None;
        self.prune();
    }

    /// Keeps only the edits of widgets the auto splitter has that change
    /// what the widget shows.
    fn prune(&mut self) {
        let widgets = &self.widgets;
        let live = &self.live;
        self.edits.retain(|key, edit| {
            widgets
                .iter()
                .find(|widget| &*widget.key == key)
                .is_some_and(|widget| value(widget, edit.as_ref()) != value(widget, live.get(key)))
        });
    }
}

/// The file dialog filters of a file selection, as LiveSplit builds them:
/// each filter's name and file extensions, then [`ALL_FILES`] except on
/// macOS, whose file dialog allows every filter's extensions at once.
///
/// - A name filter is named by its description; without one, by the MIME
///   type of its first extension ("PNG images"), or else by its extensions
///   ("SAV or BAK files"), or else by its pattern. Patterns other than
///   `*.ext` can't be given to the file dialog, so they are left out, and a
///   filter left with none is left out. So is a pattern with `;` or `|`.
/// - A MIME type filter gets the MIME type's extensions and a name from the
///   type ("Plain text files", "Images"). `*/*`, a type with no `/` and
///   unknown types are left out.
pub fn file_filters(filters: &[FileFilter]) -> Vec<(String, Vec<String>)> {
    let mut dialog: Vec<(String, Vec<String>)> = filters
        .iter()
        .filter_map(|filter| match filter {
            FileFilter::Name {
                description,
                pattern,
            } => {
                if pattern.contains([';', '|']) {
                    return None;
                }
                let extensions: Vec<String> = pattern
                    .split(' ')
                    .filter_map(extension)
                    .map(str::to_owned)
                    .collect();
                if extensions.is_empty() {
                    return None;
                }
                Some((filter_name(description.as_deref(), pattern), extensions))
            }
            FileFilter::MimeType(mime_type) => {
                let (top, sub) = mime_type.split_once('/')?;
                if top == "*" {
                    return None;
                }
                let extensions = mime_guess::get_extensions(top, sub)?;
                Some((
                    mime_description(top, sub),
                    extensions.iter().map(|&ext| ext.to_owned()).collect(),
                ))
            }
        })
        .collect();
    if cfg!(not(target_os = "macos")) {
        dialog.push((ALL_FILES.to_owned(), vec!["*".to_owned()]));
    }
    dialog
}

/// The names of a file selection's filters, in order, for the line under
/// it. A filter is named as in the file dialog, even one the file dialog
/// leaves out.
pub fn filter_names(filters: &[FileFilter]) -> Vec<String> {
    filters
        .iter()
        .map(|filter| match filter {
            FileFilter::Name {
                description,
                pattern,
            } => filter_name(description.as_deref(), pattern),
            FileFilter::MimeType(mime_type) => match mime_type.split_once('/') {
                Some((top, sub)) if top != "*" => mime_description(top, sub),
                _ => mime_type.to_string(),
            },
        })
        .collect()
}

/// The extension of a `*.ext` pattern.
fn extension(pattern: &str) -> Option<&str> {
    let (name, extension) = pattern.rsplit_once('.')?;
    (name == "*" && !extension.is_empty() && !extension.contains('*')).then_some(extension)
}

/// The name of a name filter, as LiveSplit makes it.
fn filter_name(description: Option<&str>, pattern: &str) -> String {
    if let Some(description) = description {
        return description.trim().to_owned();
    }
    let patterns: Vec<&str> = pattern.split(' ').collect();
    let mime = patterns
        .iter()
        .find_map(|pattern| mime_guess::from_ext(extension(pattern)?).first());
    if let Some(mime) = mime {
        return mime_description(mime.type_().as_str(), mime.subtype().as_str());
    }
    let extensions: Option<Vec<&str>> = patterns.iter().map(|pattern| extension(pattern)).collect();
    let Some(extensions) = extensions else {
        return pattern.trim().to_owned();
    };
    let mut name = String::new();
    for (i, extension) in extensions.iter().enumerate() {
        if i != 0 {
            name.push_str(if i + 1 != extensions.len() {
                ", "
            } else {
                " or "
            });
        }
        name.extend(extension.chars().flat_map(char::to_uppercase));
    }
    name.push_str(" files");
    name
}

/// A MIME type as words, as LiveSplit words it: "PNG images", "JSON
/// application files", "Plain text files", "Images" for `image/*`.
fn mime_description(top: &str, sub: &str) -> String {
    let mut name = String::new();
    if sub != "*" {
        // As LiveSplit does: `vnd.` only goes with an `x-` after it.
        let sub = sub
            .strip_prefix("vnd.")
            .unwrap_or(sub)
            .strip_prefix("x-")
            .unwrap_or(sub);
        let separator = |c: char| matches!(c, '-' | '.' | '+' | '|' | ' ');
        let mut chars = sub.chars();
        if let Some(first) = chars.next() {
            let first = if separator(first) { ' ' } else { first };
            name.extend(first.to_uppercase());
        }
        // Short pieces are taken for acronyms.
        for (i, piece) in chars.as_str().split(separator).enumerate() {
            if i != 0 {
                name.push(' ');
            }
            if piece.len() <= 4 - usize::from(i == 0) {
                name.extend(piece.chars().flat_map(char::to_uppercase));
            } else {
                name.push_str(piece);
            }
        }
        name.push(' ');
    }
    let mut chars = top.chars().filter(|c| *c != '|');
    if sub == "*"
        && let Some(first) = chars.next()
    {
        name.extend(first.to_uppercase());
    }
    name.extend(chars);
    name.push_str(if top == "image" { "s" } else { " files" });
    name
}

/// The value `widget` shows for `saved`, the value of its key: `saved` if it
/// is one the widget can show, otherwise its default, as the runtime does. A
/// file selection has no default.
fn value(widget: &Setting, saved: Option<&Value>) -> Option<Value> {
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
            widget(
                "level",
                WidgetKind::TextInput {
                    default_value: "prologue".into(),
                },
            ),
        ]
    }

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn editor(live: &str) -> SettingsEditor {
        let mut editor = SettingsEditor::default();
        editor.reset(table(live));
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

    fn name(description: Option<&str>, pattern: &str) -> FileFilter {
        FileFilter::Name {
            description: description.map(Into::into),
            pattern: pattern.into(),
        }
    }

    fn filter(name: &str, extensions: &[&str]) -> (String, Vec<String>) {
        (
            name.to_owned(),
            extensions.iter().map(|&ext| ext.to_owned()).collect(),
        )
    }

    /// The dialog filters, without the last one, All files.
    fn without_all_files(filters: &[FileFilter]) -> Vec<(String, Vec<String>)> {
        let mut dialog = file_filters(filters);
        if cfg!(not(target_os = "macos")) {
            assert_eq!(dialog.pop(), Some(filter(ALL_FILES, &["*"])));
        }
        dialog
    }

    #[test]
    fn widgets_show_the_runtimes_values_or_their_defaults() {
        let editor = editor("gym = true\ncategory = \"unknown\"\nlevel = 3\n");
        assert_eq!(
            editor.value(&find(&editor, "gym")),
            Some(Value::Boolean(true))
        );
        // A value the widget can't show shows the default.
        assert_eq!(
            editor.value(&find(&editor, "category")),
            Some(Value::String("any".to_owned()))
        );
        assert_eq!(
            editor.value(&find(&editor, "level")),
            Some(Value::String("prologue".to_owned()))
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
        // The runtime's map is unchanged until saving.
        assert_eq!(editor.live(), &table("other = 1\n"));

        editor.saved(Instant::now());
        assert!(!editor.is_unsaved());
        assert_eq!(editor.value(&gym), Some(Value::Boolean(true)));
        assert_eq!(
            editor.live(),
            &table("other = 1\ngym = true\nroute = \"/mnt/c/route.txt\"\n")
        );
    }

    #[test]
    fn only_edited_keys_are_written_when_saving() {
        let mut editor = editor("gym = false\nstored = 7\n");
        editor.set(&find(&editor, "gym"), Some(Value::Boolean(true)));
        editor.set(
            &find(&editor, "route"),
            Some(Value::String("/r".to_owned())),
        );
        editor.set(&find(&editor, "route"), None);
        assert_eq!(
            editor.edits(),
            [("gym".to_owned(), Some(Value::Boolean(true)))]
        );
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
        assert_eq!(editor.draft(), table("gym = true\n"));
    }

    #[test]
    fn reverting_to_defaults_is_unsaved_until_saved() {
        let mut editor = editor(
            "gym = true\ncategory = \"hundred\"\nroute = \"/mnt/c/r.txt\"\nlevel = \"x\"\nother = 1\n",
        );
        editor.revert_to_defaults();
        assert_eq!(editor.changes(), 4);
        assert_eq!(
            editor.draft(),
            table("gym = false\ncategory = \"any\"\nlevel = \"prologue\"\nother = 1\n")
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
        editor.saved(start);
        assert!(editor.saved_recently(start).is_some());
        assert!(editor.saved_recently(start + SAVED_FOR).is_none());

        editor.set(&find(&editor, "gym"), Some(Value::Boolean(true)));
        assert!(editor.saved_recently(start).is_none());
    }

    #[test]
    fn widgets_follow_values_the_auto_splitter_stores() {
        let mut editor = editor("gym = false\n");
        let now = Instant::now();
        let changed = editor.set_live(table("gym = true\nstored = 7\n"), now);
        assert_eq!(changed, ["gym", "stored"]);
        assert_eq!(
            editor.value(&find(&editor, "gym")),
            Some(Value::Boolean(true))
        );
        assert!(editor.recently_changed("stored", now));
        assert!(!editor.recently_changed("stored", now + CHANGED_FOR));
        assert!(!editor.recently_changed("category", now));
    }

    #[test]
    fn a_removed_value_counts_as_changed() {
        let mut editor = editor("gym = false\nstored = 7\n");
        let changed = editor.set_live(table("gym = false\n"), Instant::now());
        assert_eq!(changed, ["stored"]);
    }

    #[test]
    fn an_unsaved_edit_shows_over_a_stored_value_until_saved() {
        let mut editor = editor("level = \"a\"\n");
        let level = find(&editor, "level");
        editor.set(&level, Some(Value::String("mine".to_owned())));
        editor.set_live(table("level = \"theirs\"\n"), Instant::now());
        assert_eq!(editor.value(&level), Some(Value::String("mine".to_owned())));
        assert!(editor.is_unsaved());

        // Saving writes the edit over it: the last writer wins.
        editor.saved(Instant::now());
        assert_eq!(editor.live(), &table("level = \"mine\"\n"));
    }

    #[test]
    fn an_edit_the_auto_splitter_then_stores_itself_is_no_longer_unsaved() {
        let mut editor = editor("gym = false\n");
        editor.set(&find(&editor, "gym"), Some(Value::Boolean(true)));
        editor.set_live(table("gym = true\n"), Instant::now());
        assert!(!editor.is_unsaved());
    }

    #[test]
    fn edits_of_settings_the_auto_splitter_drops_are_dropped() {
        let mut editor = editor("");
        editor.set(&find(&editor, "gym"), Some(Value::Boolean(true)));
        let mut widgets = widgets();
        widgets.retain(|widget| &*widget.key != "gym");
        editor.set_widgets(widgets);
        assert!(!editor.is_unsaved());
    }

    #[test]
    fn widgets_arrive_after_a_reset_and_stay_when_the_game_changes() {
        let mut editor = editor("");
        editor.reset_keeping_widgets(table("gym = true\n"));
        assert_eq!(editor.widgets().len(), 5);
        assert!(!editor.is_unsaved());
        editor.reset(Table::new());
        assert!(editor.widgets().is_empty());
        assert!(editor.keys().is_empty());
    }

    #[test]
    fn named_filters_keep_their_names_and_extension_patterns() {
        let filters = [
            name(Some(" Images "), "*.png *.jpg"),
            name(Some("Rust"), "*.rs Cargo.*"),
        ];
        assert_eq!(
            without_all_files(&filters),
            [filter("Images", &["png", "jpg"]), filter("Rust", &["rs"]),]
        );
    }

    #[test]
    fn unnamed_filters_are_named_as_livesplit_names_them() {
        let filters = [
            name(None, "*.png"),
            name(None, "*.json"),
            name(None, "*.zzsav *.zzbak *.zzold"),
            name(None, "*.zzsav *.zzbak"),
        ];
        assert_eq!(
            without_all_files(&filters),
            [
                filter("PNG images", &["png"]),
                filter("JSON application files", &["json"]),
                filter("ZZSAV, ZZBAK or ZZOLD files", &["zzsav", "zzbak", "zzold"]),
                filter("ZZSAV or ZZBAK files", &["zzsav", "zzbak"]),
            ]
        );
    }

    #[test]
    fn patterns_that_are_not_extensions_are_left_out() {
        let filters = [
            name(None, "save_*.dat"),
            name(Some("Any"), "*"),
            name(Some("Split"), "*.a;*.b"),
            name(Some("Pipe"), "*.a|b"),
        ];
        assert_eq!(without_all_files(&filters), []);
    }

    #[test]
    fn mime_types_become_their_extensions() {
        let filters = [
            FileFilter::MimeType("text/plain".into()),
            FileFilter::MimeType("image/*".into()),
            FileFilter::MimeType("*/*".into()),
            FileFilter::MimeType("nonsense".into()),
            FileFilter::MimeType("application/x-unknown-to-anyone".into()),
        ];
        let dialog = without_all_files(&filters);
        assert_eq!(dialog.len(), 2, "{dialog:?}");
        assert_eq!(dialog[0].0, "Plain text files");
        assert!(dialog[0].1.contains(&"txt".to_owned()), "{dialog:?}");
        assert_eq!(dialog[1].0, "Images");
        for extension in ["png", "jpg", "gif"] {
            assert!(dialog[1].1.contains(&extension.to_owned()), "{dialog:?}");
        }
    }

    #[test]
    fn with_no_filters_every_file_can_be_picked() {
        assert_eq!(without_all_files(&[]), []);
    }

    #[test]
    fn mime_types_are_worded_as_livesplit_words_them() {
        assert_eq!(mime_description("image", "png"), "PNG images");
        assert_eq!(mime_description("image", "svg+xml"), "SVG XML images");
        assert_eq!(mime_description("text", "plain"), "Plain text files");
        assert_eq!(mime_description("image", "*"), "Images");
        assert_eq!(mime_description("audio", "*"), "Audio files");
        assert_eq!(
            mime_description("application", "x-tar"),
            "TAR application files"
        );
        // LiveSplit strips `vnd.` only with an `x-` after it.
        assert_eq!(
            mime_description("application", "vnd.x-thing"),
            "Thing application files"
        );
        assert_eq!(
            mime_description("application", "vnd.ms-excel"),
            "VND MS excel application files"
        );
    }

    #[test]
    fn filter_names_list_every_filter_in_order() {
        let filters = [
            name(Some("Images"), "*.png *.jpg"),
            name(None, "*.json"),
            name(None, "save_*.dat"),
            FileFilter::MimeType("image/*".into()),
            FileFilter::MimeType("*/*".into()),
        ];
        assert_eq!(
            filter_names(&filters),
            [
                "Images",
                "JSON application files",
                "save_*.dat",
                "Images",
                "*/*"
            ]
        );
    }
}
