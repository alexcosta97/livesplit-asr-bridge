//! `app.toml`: the app's own settings (spec §7.1), read at start-up.
//!
//! ```toml
//! port = 16834
//! ```
//!
//! Only the port is stored so far. Keys this version doesn't know, for
//! example ones written by a newer version, are kept when the file is saved.

use toml::{Table, Value};

use super::{Config, read_if_exists, write_replacing};

/// The server's port when none is saved: the same default as the original
/// LiveSplit server (spec §6.5).
pub const DEFAULT_PORT: u16 = 16834;

const PORT: &str = "port";

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
    fn a_broken_file_is_an_error() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir()).unwrap();
        fs::write(config.dir().join("app.toml"), "port = ").unwrap();
        let error = AppSettings::load(&config).unwrap_err();
        assert!(error.contains("app.toml"), "{error}");
    }
}
