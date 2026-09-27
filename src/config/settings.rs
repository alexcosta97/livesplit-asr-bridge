//! A game's saved settings and the merge rules of spec §7.2.
//!
//! A game's file holds one map of setting keys to values, shared by every
//! auto splitter associated with the game:
//!
//! - **Loading:** the auto splitter starts with the game's map. It reads the
//!   values of the keys it publishes as widgets, and the runtime ignores a
//!   saved value whose type doesn't match the widget. Other keys are
//!   ignored.
//! - **Saving:** only the loaded auto splitter's keys are written. Other
//!   keys are kept unchanged.
//! - **Revert to defaults:** only the loaded auto splitter's keys change.

use std::sync::Arc;

use livesplit_auto_splitting::settings::{self, Widget, WidgetKind};
use toml::{Table, Value};

/// A key the loaded auto splitter publishes as a setting, with its default
/// value. A file selection has no default: reverting removes its value.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingKey {
    pub key: String,
    pub default: Option<Value>,
}

impl SettingKey {
    /// The keys of the settings `widgets` publish, in order. Headings have a
    /// key but no value, so they aren't settings.
    #[expect(dead_code, reason = "the Settings tab saves and reverts (#8)")]
    pub fn from_widgets(widgets: &[Widget]) -> Vec<Self> {
        widgets
            .iter()
            .filter_map(|widget| Self::new(&widget.key, &widget.kind))
            .collect()
    }

    /// The setting a widget of `kind` publishes under `key`, if it is one.
    fn new(key: &str, kind: &WidgetKind) -> Option<Self> {
        let default = match kind {
            WidgetKind::Title { .. } => return None,
            WidgetKind::Bool { default_value } => Some(Value::Boolean(*default_value)),
            WidgetKind::Choice {
                default_option_key, ..
            } => Some(Value::String(default_option_key.to_string())),
            WidgetKind::TextInput { default_value } => {
                Some(Value::String(default_value.to_string()))
            }
            WidgetKind::FileSelect { .. } => None,
        };
        Some(Self {
            key: key.to_owned(),
            default,
        })
    }
}

/// The game's saved settings after saving `current`, the loaded auto
/// splitter's settings: its keys take their values from `current` (a key
/// missing there is removed), and every other saved key is kept.
#[cfg_attr(not(test), expect(dead_code, reason = "the Settings tab saves (#8)"))]
pub fn merge_saved(saved: &Table, current: &Table, keys: &[SettingKey]) -> Table {
    let mut merged = saved.clone();
    for SettingKey { key, .. } in keys {
        match current.get(key) {
            Some(value) => {
                merged.insert(key.clone(), value.clone());
            }
            None => {
                merged.remove(key);
            }
        }
    }
    merged
}

/// `draft` with the loaded auto splitter's keys set to their defaults. Other
/// keys are unchanged.
#[cfg_attr(not(test), expect(dead_code, reason = "the Settings tab reverts (#8)"))]
pub fn revert_to_defaults(draft: &Table, keys: &[SettingKey]) -> Table {
    let mut reverted = draft.clone();
    for SettingKey { key, default } in keys {
        match default {
            Some(value) => {
                reverted.insert(key.clone(), value.clone());
            }
            None => {
                reverted.remove(key);
            }
        }
    }
    reverted
}

/// Saved settings as the runtime's settings map. TOML dates have no
/// runtime equivalent and are left out.
pub fn to_runtime(table: &Table) -> settings::Map {
    let mut map = settings::Map::new();
    for (key, value) in table {
        if let Some(value) = value_to_runtime(value) {
            map.insert(Arc::from(key.as_str()), value);
        }
    }
    map
}

fn value_to_runtime(value: &Value) -> Option<settings::Value> {
    Some(match value {
        Value::Boolean(value) => settings::Value::Bool(*value),
        Value::Integer(value) => settings::Value::I64(*value),
        Value::Float(value) => settings::Value::F64(*value),
        Value::String(value) => settings::Value::String(Arc::from(value.as_str())),
        Value::Array(values) => {
            let mut list = settings::List::new();
            for value in values.iter().filter_map(value_to_runtime) {
                list.push(value);
            }
            settings::Value::List(list)
        }
        Value::Table(table) => settings::Value::Map(to_runtime(table)),
        Value::Datetime(_) => return None,
    })
}

/// The runtime's settings map as settings to save. Kinds of value the
/// runtime may add later are left out.
#[cfg_attr(not(test), expect(dead_code, reason = "the Settings tab saves (#8)"))]
pub fn from_runtime(map: &settings::Map) -> Table {
    map.iter()
        .filter_map(|(key, value)| Some((key.to_owned(), value_from_runtime(value)?)))
        .collect()
}

fn value_from_runtime(value: &settings::Value) -> Option<Value> {
    Some(match value {
        settings::Value::Bool(value) => Value::Boolean(*value),
        settings::Value::I64(value) => Value::Integer(*value),
        settings::Value::F64(value) => Value::Float(*value),
        settings::Value::String(value) => Value::String(value.to_string()),
        settings::Value::List(list) => {
            Value::Array(list.iter().filter_map(value_from_runtime).collect())
        }
        settings::Value::Map(map) => Value::Table(from_runtime(map)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn key(key: &str, default: Option<Value>) -> SettingKey {
        SettingKey {
            key: key.to_owned(),
            default,
        }
    }

    #[test]
    fn saving_writes_only_the_loaded_auto_splitters_keys() {
        let saved = table("gym = false\nother_splitter = 3\n");
        let current = table("gym = true\nnot_published = \"x\"\n");
        let keys = [key("gym", Some(Value::Boolean(false)))];
        assert_eq!(
            merge_saved(&saved, &current, &keys),
            table("gym = true\nother_splitter = 3\n")
        );
    }

    #[test]
    fn saving_keeps_keys_the_loaded_auto_splitter_does_not_recognise() {
        let saved = table("other_splitter = \"keep me\"\n[nested]\nx = 1\n");
        let current = table("gym = true\n");
        let keys = [key("gym", None)];
        let merged = merge_saved(&saved, &current, &keys);
        assert_eq!(merged["other_splitter"].as_str(), Some("keep me"));
        assert_eq!(merged["nested"], table("x = 1\n").into());
        assert_eq!(merged["gym"].as_bool(), Some(true));
    }

    #[test]
    fn saving_removes_a_recognised_key_that_has_no_value() {
        let saved = table("route_file = \"/old.txt\"\nother = 1\n");
        let keys = [key("route_file", None)];
        assert_eq!(
            merge_saved(&saved, &Table::new(), &keys),
            table("other = 1\n")
        );
    }

    #[test]
    fn reverting_only_affects_the_loaded_auto_splitters_keys() {
        let draft = table("gym = true\ncategory = \"any%\"\nroute_file = \"/r.txt\"\nother = 1\n");
        let keys = [
            key("gym", Some(Value::Boolean(false))),
            key("category", Some(Value::String("100%".to_owned()))),
            key("route_file", None),
        ];
        assert_eq!(
            revert_to_defaults(&draft, &keys),
            table("gym = false\ncategory = \"100%\"\nother = 1\n")
        );
    }

    #[test]
    fn reverting_adds_defaults_for_keys_never_saved() {
        let keys = [key("gym", Some(Value::Boolean(true)))];
        assert_eq!(
            revert_to_defaults(&Table::new(), &keys),
            table("gym = true\n")
        );
    }

    #[test]
    fn setting_keys_come_from_every_widget_but_headings() {
        let title = WidgetKind::Title { heading_level: 0 };
        assert_eq!(SettingKey::new("heading", &title), None);
        let check = WidgetKind::Bool {
            default_value: true,
        };
        assert_eq!(
            SettingKey::new("gym", &check),
            Some(key("gym", Some(Value::Boolean(true))))
        );
        let choice = WidgetKind::Choice {
            default_option_key: "any".into(),
            options: Arc::new(Vec::new()),
        };
        assert_eq!(
            SettingKey::new("category", &choice),
            Some(key("category", Some(Value::String("any".to_owned()))))
        );
        let text = WidgetKind::TextInput {
            default_value: "hi".into(),
        };
        assert_eq!(
            SettingKey::new("greeting", &text),
            Some(key("greeting", Some(Value::String("hi".to_owned()))))
        );
        let file = WidgetKind::FileSelect {
            filters: Arc::new(Vec::new()),
        };
        assert_eq!(SettingKey::new("route", &file), Some(key("route", None)));
    }

    #[test]
    fn settings_convert_to_the_runtime_and_back() {
        let saved = table(
            "flag = true\ncount = 3\nratio = 0.5\nname = \"x\"\nlist = [1, \"two\"]\n\
             [nested]\ninner = false\n",
        );
        let runtime = to_runtime(&saved);
        assert_eq!(
            runtime.get("flag").and_then(settings::Value::to_bool),
            Some(true)
        );
        assert_eq!(from_runtime(&runtime), saved);
    }

    #[test]
    fn dates_have_no_runtime_value_and_are_left_out() {
        let saved = table("when = 1979-05-27\nflag = true\n");
        let runtime = to_runtime(&saved);
        assert_eq!(runtime.len(), 1);
        assert!(runtime.get("when").is_none());
    }
}
