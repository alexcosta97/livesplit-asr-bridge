//! `splitters.toml`: which game each known `.wasm` is for (spec §7.3).
//!
//! ```toml
//! [splitters]
//! "/home/me/splitters/gta_sa_de_autosplitter.wasm" = "gta-san-andreas-definitive-edition"
//! ```
//!
//! Each file's path maps to the slug of its game, which names the game's
//! file in `games/`.

use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use super::{Config, read_if_exists, write_replacing};

/// The contents of `splitters.toml`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Splitters {
    /// Each known `.wasm` path and the slug of its game.
    #[serde(default)]
    splitters: BTreeMap<String, String>,
}

impl Splitters {
    /// Reads `splitters.toml`. A missing file means no auto splitter is known
    /// yet.
    pub fn load(config: &Config) -> Result<Self, String> {
        let path = config.splitters_file();
        match read_if_exists(&path)? {
            None => Ok(Self::default()),
            Some(text) => toml::from_str(&text)
                .map_err(|error| format!("couldn't read {}: {error}", path.display())),
        }
    }

    /// Writes `splitters.toml`.
    pub fn save(&self, config: &Config) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|error| error.to_string())?;
        write_replacing(&config.splitters_file(), &text)
    }

    /// The slug of the game the auto splitter at `path` is for, if it is
    /// known.
    pub fn game_of(&self, path: &Path) -> Option<&str> {
        self.splitters.get(key(path).ok()?).map(String::as_str)
    }

    /// Remembers that the auto splitter at `path` is for the game `slug`,
    /// replacing any earlier game. A path that isn't valid Unicode can't be
    /// written to the file, so it can't be remembered.
    pub fn associate(&mut self, path: &Path, slug: &str) -> Result<(), String> {
        let key = key(path)?;
        self.splitters.insert(key.to_owned(), slug.to_owned());
        Ok(())
    }

    /// How many known auto splitters are for the game `slug`.
    pub fn count_for(&self, slug: &str) -> usize {
        self.splitters.values().filter(|game| *game == slug).count()
    }

    /// The known auto splitters' paths, for tests.
    #[cfg(test)]
    fn paths(&self) -> Vec<std::path::PathBuf> {
        self.splitters.keys().map(Into::into).collect()
    }
}

/// The key a path is stored under: the path as it is.
fn key(path: &Path) -> Result<&str, String> {
    path.to_str().ok_or_else(|| {
        format!(
            "can't remember the game for {}: its path isn't valid Unicode",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn config() -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        (dir, config)
    }

    #[test]
    fn with_no_file_every_auto_splitter_is_new() {
        let (_dir, config) = config();
        let splitters = Splitters::load(&config).unwrap();
        assert_eq!(splitters.game_of(Path::new("/s/gta.wasm")), None);
    }

    #[test]
    fn a_known_file_finds_its_game_and_a_new_one_does_not() {
        let (_dir, config) = config();
        let mut splitters = Splitters::default();
        splitters
            .associate(Path::new("/s/gta.wasm"), "gta-san-andreas")
            .unwrap();
        splitters.save(&config).unwrap();

        let splitters = Splitters::load(&config).unwrap();
        assert_eq!(
            splitters.game_of(Path::new("/s/gta.wasm")),
            Some("gta-san-andreas")
        );
        assert_eq!(splitters.game_of(Path::new("/s/other.wasm")), None);
        assert_eq!(splitters.game_of(Path::new("/elsewhere/gta.wasm")), None);
    }

    #[test]
    fn the_association_can_be_changed() {
        let (_dir, config) = config();
        let path = Path::new("/s/gta.wasm");
        let mut splitters = Splitters::default();
        splitters.associate(path, "gta-san-andreas").unwrap();
        splitters.associate(path, "gta-vice-city").unwrap();
        splitters.save(&config).unwrap();
        let splitters = Splitters::load(&config).unwrap();
        assert_eq!(splitters.game_of(path), Some("gta-vice-city"));
        assert_eq!(splitters.paths(), [PathBuf::from("/s/gta.wasm")]);
    }

    #[test]
    fn counts_the_auto_splitters_of_each_game() {
        let mut splitters = Splitters::default();
        splitters.associate(Path::new("/a.wasm"), "gta").unwrap();
        splitters.associate(Path::new("/b.wasm"), "gta").unwrap();
        splitters.associate(Path::new("/c.wasm"), "portal").unwrap();
        assert_eq!(splitters.count_for("gta"), 2);
        assert_eq!(splitters.count_for("portal"), 1);
        assert_eq!(splitters.count_for("celeste"), 0);
    }

    #[test]
    fn the_file_is_readable_toml() {
        let (_dir, config) = config();
        let mut splitters = Splitters::default();
        splitters
            .associate(Path::new("/s/gta sa.wasm"), "gta-san-andreas")
            .unwrap();
        splitters.save(&config).unwrap();
        let text = std::fs::read_to_string(config.splitters_file()).unwrap();
        assert_eq!(
            text,
            "[splitters]\n\"/s/gta sa.wasm\" = \"gta-san-andreas\"\n"
        );
    }

    #[test]
    fn a_broken_file_is_an_error() {
        let (_dir, config) = config();
        write_replacing(&config.splitters_file(), "not toml [").unwrap();
        let error = Splitters::load(&config).unwrap_err();
        assert!(error.contains("splitters.toml"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_unicode_is_not_remembered() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let path = Path::new(OsStr::from_bytes(b"/s/\xff.wasm"));
        let mut splitters = Splitters::default();
        assert!(splitters.associate(path, "gta").is_err());
        assert_eq!(splitters.game_of(path), None);
    }
}
