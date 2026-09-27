//! `app.toml`: the app's own settings (spec §7.1), read at start-up.
//!
//! ```toml
//! port = 16834
//!
//! [log_filters]
//! auto_splitter = true
//! connection = false
//! app = false
//!
//! [window]
//! remember = true
//! width = 800.0
//! height = 600.0
//! x = 120.0
//! y = 80.0
//! ```
//!
//! Keys this version doesn't know, for example ones written by a newer
//! version, are kept when the file is saved.

use toml::{Table, Value};

use super::{Config, read_if_exists, write_replacing};
use crate::logging::Filters;

/// The server's port when none is saved: the same default as the original
/// LiveSplit server (spec §6.5).
pub const DEFAULT_PORT: u16 = 16834;

const PORT: &str = "port";
const LOG_FILTERS: &str = "log_filters";
const AUTO_SPLITTER: &str = "auto_splitter";
const CONNECTION: &str = "connection";
const APP: &str = "app";
const WINDOW: &str = "window";
const REMEMBER: &str = "remember";
const WIDTH: &str = "width";
const HEIGHT: &str = "height";
const X: &str = "x";
const Y: &str = "y";

/// The window's size and position, in points, as last seen. Either can be
/// unknown: on Wayland, for example, the app can't read or set the window's
/// position.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WindowGeometry {
    /// The size of the window's contents: width, height.
    pub size: Option<[f32; 2]>,
    /// The position of the window's outer corner: x, y.
    pub position: Option<[f32; 2]>,
}

/// The contents of `app.toml`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AppSettings {
    table: Table,
}

impl AppSettings {
    /// Reads `app.toml`. A missing file means nothing is saved yet.
    pub fn load(config: &Config) -> Result<Self, String> {
        let path = config.app_file();
        let table = match read_if_exists(&path)? {
            None => Table::new(),
            Some(text) => text
                .parse()
                .map_err(|error| format!("couldn't read {}: {error}", path.display()))?,
        };
        Ok(Self { table })
    }

    /// Writes `app.toml`.
    pub fn save(&self, config: &Config) -> Result<(), String> {
        write_replacing(&config.app_file(), &self.table.to_string())
    }

    /// The server's port: the saved one, or [`DEFAULT_PORT`] if none is
    /// saved or the saved value isn't a port.
    pub fn port(&self) -> u16 {
        match self.table.get(PORT) {
            Some(Value::Integer(port)) => u16::try_from(*port)
                .ok()
                .filter(|port| *port != 0)
                .unwrap_or(DEFAULT_PORT),
            _ => DEFAULT_PORT,
        }
    }

    /// Sets the port to save.
    pub fn set_port(&mut self, port: u16) {
        self.table
            .insert(PORT.to_owned(), Value::Integer(i64::from(port)));
    }

    /// The Log tab's filters: the saved ones, or only errors if none are
    /// saved. A filter that isn't saved, or isn't true or false, is off.
    pub fn log_filters(&self) -> Filters {
        let Some(Value::Table(filters)) = self.table.get(LOG_FILTERS) else {
            return Filters::default();
        };
        let ticked = |key: &str| filters.get(key).and_then(Value::as_bool) == Some(true);
        Filters {
            auto_splitter: ticked(AUTO_SPLITTER),
            connection: ticked(CONNECTION),
            app: ticked(APP),
        }
    }

    /// Sets the Log tab's filters to save.
    pub fn set_log_filters(&mut self, filters: Filters) {
        let table = Table::from_iter([
            (
                AUTO_SPLITTER.to_owned(),
                Value::Boolean(filters.auto_splitter),
            ),
            (CONNECTION.to_owned(), Value::Boolean(filters.connection)),
            (APP.to_owned(), Value::Boolean(filters.app)),
        ]);
        self.table
            .insert(LOG_FILTERS.to_owned(), Value::Table(table));
    }

    /// Whether the window's size and position are remembered (spec §6.7):
    /// on unless it is saved as off.
    pub fn remember_window(&self) -> bool {
        self.window()
            .and_then(|window| window.get(REMEMBER))
            .and_then(Value::as_bool)
            != Some(false)
    }

    /// Sets whether the window's size and position are remembered.
    pub fn set_remember_window(&mut self, remember: bool) {
        self.window_mut()
            .insert(REMEMBER.to_owned(), Value::Boolean(remember));
    }

    /// The window's saved size and position. A size or position that isn't
    /// saved, or isn't numbers, is unknown, and so is a size that isn't
    /// positive.
    pub fn window_geometry(&self) -> WindowGeometry {
        let Some(window) = self.window() else {
            return WindowGeometry::default();
        };
        let pair = |a: &str, b: &str| Some([number(window.get(a)?)?, number(window.get(b)?)?]);
        WindowGeometry {
            size: pair(WIDTH, HEIGHT).filter(|[width, height]| *width > 0.0 && *height > 0.0),
            position: pair(X, Y),
        }
    }

    /// Sets the window's size and position to save. What is unknown keeps
    /// its saved value.
    pub fn set_window_geometry(&mut self, geometry: WindowGeometry) {
        let window = self.window_mut();
        let mut insert = |key: &str, value: f32| {
            window.insert(key.to_owned(), Value::Float(f64::from(value).round()));
        };
        if let Some([width, height]) = geometry.size {
            insert(WIDTH, width);
            insert(HEIGHT, height);
        }
        if let Some([x, y]) = geometry.position {
            insert(X, x);
            insert(Y, y);
        }
    }

    fn window(&self) -> Option<&Table> {
        self.table.get(WINDOW).and_then(Value::as_table)
    }

    /// The `[window]` table, replacing a `window` key that isn't one.
    fn window_mut(&mut self) -> &mut Table {
        let window = self
            .table
            .entry(WINDOW)
            .or_insert_with(|| Value::Table(Table::new()));
        if !window.is_table() {
            *window = Value::Table(Table::new());
        }
        window
            .as_table_mut()
            .expect("the window key was made a table")
    }
}

/// A finite number, written as an integer or a float.
fn number(value: &Value) -> Option<f32> {
    let number = match value {
        Value::Integer(number) => *number as f32,
        Value::Float(number) => *number as f32,
        _ => return None,
    };
    number.is_finite().then_some(number)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn config() -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        (dir, config)
    }

    #[test]
    fn with_no_file_the_port_is_the_default() {
        let (_dir, config) = config();
        assert_eq!(AppSettings::load(&config).unwrap().port(), 16834);
    }

    #[test]
    fn a_saved_port_is_used_on_the_next_start() {
        let (_dir, config) = config();
        let mut settings = AppSettings::load(&config).unwrap();
        settings.set_port(20000);
        settings.save(&config).unwrap();
        assert_eq!(
            fs::read_to_string(config.dir().join("app.toml")).unwrap(),
            "port = 20000\n"
        );
        assert_eq!(AppSettings::load(&config).unwrap().port(), 20000);
    }

    #[test]
    fn a_value_that_is_not_a_port_reads_as_the_default() {
        let (_dir, config) = config();
        for text in ["port = 0", "port = 70000", "port = -1", "port = \"16835\""] {
            fs::create_dir_all(config.dir()).unwrap();
            fs::write(config.dir().join("app.toml"), text).unwrap();
            assert_eq!(
                AppSettings::load(&config).unwrap().port(),
                DEFAULT_PORT,
                "{text}"
            );
        }
    }

    #[test]
    fn saving_keeps_keys_this_version_does_not_know() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.dir().join("app.toml"), "later = true\nport = 1\n").unwrap();
        let mut settings = AppSettings::load(&config).unwrap();
        settings.set_port(20000);
        settings.save(&config).unwrap();
        let saved = AppSettings::load(&config).unwrap();
        assert_eq!(saved.port(), 20000);
        assert_eq!(saved.table["later"].as_bool(), Some(true));
    }

    #[test]
    fn with_no_file_only_errors_are_shown() {
        let (_dir, config) = config();
        assert_eq!(
            AppSettings::load(&config).unwrap().log_filters(),
            Filters::default()
        );
    }

    #[test]
    fn saved_log_filters_are_used_on_the_next_start() {
        let (_dir, config) = config();
        let mut settings = AppSettings::load(&config).unwrap();
        let filters = Filters {
            auto_splitter: true,
            connection: false,
            app: true,
        };
        settings.set_log_filters(filters);
        settings.save(&config).unwrap();
        assert_eq!(
            fs::read_to_string(config.dir().join("app.toml")).unwrap(),
            "[log_filters]\nauto_splitter = true\nconnection = false\napp = true\n"
        );
        assert_eq!(AppSettings::load(&config).unwrap().log_filters(), filters);
    }

    #[test]
    fn log_filters_that_are_not_true_or_false_are_off() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(
            config.dir().join("app.toml"),
            "[log_filters]\nauto_splitter = 1\nconnection = true\n",
        )
        .unwrap();
        assert_eq!(
            AppSettings::load(&config).unwrap().log_filters(),
            Filters {
                connection: true,
                ..Filters::default()
            }
        );
    }

    #[test]
    fn with_no_file_the_window_is_remembered_but_nothing_is_saved() {
        let (_dir, config) = config();
        let settings = AppSettings::load(&config).unwrap();
        assert!(settings.remember_window());
        assert_eq!(settings.window_geometry(), WindowGeometry::default());
    }

    #[test]
    fn the_window_preference_and_geometry_are_used_on_the_next_start() {
        let (_dir, config) = config();
        let mut settings = AppSettings::load(&config).unwrap();
        settings.set_remember_window(false);
        let geometry = WindowGeometry {
            size: Some([1024.0, 700.4]),
            position: Some([-1280.0, 40.0]),
        };
        settings.set_window_geometry(geometry);
        settings.save(&config).unwrap();
        assert_eq!(
            fs::read_to_string(config.dir().join("app.toml")).unwrap(),
            "[window]\nremember = false\nwidth = 1024.0\nheight = 700.0\nx = -1280.0\ny = 40.0\n"
        );
        let saved = AppSettings::load(&config).unwrap();
        assert!(!saved.remember_window());
        assert_eq!(
            saved.window_geometry(),
            WindowGeometry {
                size: Some([1024.0, 700.0]),
                position: Some([-1280.0, 40.0]),
            }
        );
    }

    #[test]
    fn an_unknown_position_keeps_the_saved_one() {
        let (_dir, config) = config();
        let mut settings = AppSettings::load(&config).unwrap();
        settings.set_window_geometry(WindowGeometry {
            size: Some([800.0, 600.0]),
            position: Some([10.0, 20.0]),
        });
        settings.set_window_geometry(WindowGeometry {
            size: Some([900.0, 650.0]),
            position: None,
        });
        assert_eq!(
            settings.window_geometry(),
            WindowGeometry {
                size: Some([900.0, 650.0]),
                position: Some([10.0, 20.0]),
            }
        );
    }

    #[test]
    fn window_values_that_are_not_numbers_are_unknown() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(
            config.dir().join("app.toml"),
            "[window]\nremember = \"no\"\nwidth = 0\nheight = 600\nx = 5\ny = nan\n",
        )
        .unwrap();
        let settings = AppSettings::load(&config).unwrap();
        assert!(settings.remember_window());
        assert_eq!(settings.window_geometry(), WindowGeometry::default());
    }

    #[test]
    fn a_window_key_that_is_not_a_table_is_replaced() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.dir().join("app.toml"), "window = 3\n").unwrap();
        let mut settings = AppSettings::load(&config).unwrap();
        assert!(settings.remember_window());
        settings.set_remember_window(false);
        assert!(!settings.remember_window());
    }

    #[test]
    fn a_broken_file_is_an_error() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.dir().join("app.toml"), "port = ").unwrap();
        let error = AppSettings::load(&config).unwrap_err();
        assert!(error.contains("app.toml"), "{error}");
    }
}
