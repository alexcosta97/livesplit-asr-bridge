//! `games/<game-slug>.toml`: a game's display name and saved auto splitter
//! settings.
//!
//! ```toml
//! name = "GTA San Andreas — Definitive Edition"
//!
//! [settings]
//! start_on_new_game = true
//! ```

use std::fs;

use serde::{Deserialize, Serialize};
use toml::Table;

use super::{Config, SettingKey, Splitters, merge_saved, read_if_exists, slug, write_replacing};

/// A game and its saved settings.
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    /// Names the game's file.
    pub slug: String,
    /// The name shown to people.
    pub name: String,
    /// The saved settings, shared by every auto splitter for the game.
    pub settings: Table,
}

/// A game in the list the Game dialog offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSummary {
    pub slug: String,
    pub name: String,
    /// How many known auto splitters are for this game.
    pub splitters: usize,
}

/// The contents of a game's file.
#[derive(Debug, Serialize, Deserialize)]
struct GameFile {
    name: String,
    #[serde(default)]
    settings: Table,
}

impl Game {
    /// Reads the game `slug`'s file. A missing file, for example one deleted
    /// by hand, is a game with that slug as its name and no saved settings.
    pub fn load(config: &Config, slug: &str) -> Result<Self, String> {
        let path = config.game_file(slug);
        let Some(text) = read_if_exists(&path)? else {
            return Ok(Self {
                slug: slug.to_owned(),
                name: slug.to_owned(),
                settings: Table::new(),
            });
        };
        let file: GameFile = toml::from_str(&text)
            .map_err(|error| format!("couldn't read {}: {error}", path.display()))?;
        Ok(Self {
            slug: slug.to_owned(),
            name: file.name,
            settings: file.settings,
        })
    }

    /// Sets up a new game called `name`, with no saved settings, and writes
    /// its file. Its slug is made from the name, with a number added if
    /// another game already has that slug.
    pub fn create(config: &Config, name: &str) -> Result<Self, String> {
        let base = slug(name);
        let slug = (1..)
            .map(|n| match n {
                1 => base.clone(),
                n => format!("{base}-{n}"),
            })
            .find(|slug| !config.game_file(slug).exists())
            .expect("an unused slug");
        let game = Self {
            slug,
            name: name.trim().to_owned(),
            settings: Table::new(),
        };
        game.write(config)?;
        Ok(game)
    }

    /// Saves `current`, the loaded auto splitter's settings, with the merge
    /// rules of spec §7.2: only the keys in `keys` are written, and other
    /// saved keys are kept. The file is read again first, so the saved
    /// settings are the latest.
    #[cfg_attr(not(test), expect(dead_code, reason = "the Settings tab saves (#8)"))]
    pub fn save_settings(
        config: &Config,
        slug: &str,
        current: &Table,
        keys: &[SettingKey],
    ) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        game.settings = merge_saved(&game.settings, current, keys);
        game.write(config)?;
        Ok(game)
    }

    fn write(&self, config: &Config) -> Result<(), String> {
        let file = GameFile {
            name: self.name.clone(),
            settings: self.settings.clone(),
        };
        let text = toml::to_string(&file).map_err(|error| error.to_string())?;
        write_replacing(&config.game_file(&self.slug), &text)
    }

    /// The games already set up, by name, with how many auto splitters use
    /// each, for the Game dialog. Reads the name from each game's file;
    /// files that can't be read are reported and left out.
    pub fn list(config: &Config, splitters: &Splitters) -> (Vec<GameSummary>, Vec<String>) {
        let mut games = Vec::new();
        let mut errors = Vec::new();
        let entries = match fs::read_dir(config.games_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return (games, errors);
            }
            Err(error) => {
                errors.push(format!(
                    "couldn't list {}: {error}",
                    config.games_dir().display()
                ));
                return (games, errors);
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "toml") {
                continue;
            }
            let Some(slug) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            match Self::load(config, slug) {
                Ok(game) => games.push(GameSummary {
                    splitters: splitters.count_for(&game.slug),
                    slug: game.slug,
                    name: game.name,
                }),
                Err(error) => errors.push(error),
            }
        }
        games.sort_by_cached_key(|game| (game.name.to_lowercase(), game.slug.clone()));
        (games, errors)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use toml::Value;

    use super::*;

    fn config() -> (tempfile::TempDir, Config) {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        (dir, config)
    }

    #[test]
    fn a_new_game_gets_a_file_named_after_its_slug() {
        let (_dir, config) = config();
        let game = Game::create(&config, "  GTA San Andreas — DE ").unwrap();
        assert_eq!(game.slug, "gta-san-andreas-de");
        assert_eq!(game.name, "GTA San Andreas — DE");
        let text = fs::read_to_string(config.dir().join("games/gta-san-andreas-de.toml")).unwrap();
        assert_eq!(text, "name = \"GTA San Andreas — DE\"\n\n[settings]\n");
        assert_eq!(Game::load(&config, "gta-san-andreas-de").unwrap(), game);
    }

    #[test]
    fn games_whose_names_share_a_slug_get_their_own_files() {
        let (_dir, config) = config();
        let first = Game::create(&config, "Portal: 2").unwrap();
        let second = Game::create(&config, "Portal 2").unwrap();
        assert_eq!(first.slug, "portal-2");
        assert_eq!(second.slug, "portal-2-2");
        assert_eq!(Game::load(&config, "portal-2").unwrap().name, "Portal: 2");
    }

    #[test]
    fn a_missing_file_is_a_game_with_no_settings() {
        let (_dir, config) = config();
        let game = Game::load(&config, "gone").unwrap();
        assert_eq!(game.name, "gone");
        assert!(game.settings.is_empty());
    }

    #[test]
    fn saving_settings_keeps_keys_the_auto_splitter_does_not_recognise() {
        let (_dir, config) = config();
        let game = Game::create(&config, "GTA").unwrap();
        let mut earlier = Table::new();
        earlier.insert("other_splitter".to_owned(), Value::Integer(3));
        earlier.insert("gym".to_owned(), Value::Boolean(false));
        let keys_of_other = [SettingKey {
            key: "other_splitter".to_owned(),
            default: None,
        }];
        Game::save_settings(&config, &game.slug, &earlier, &keys_of_other).unwrap();

        let mut current = Table::new();
        current.insert("gym".to_owned(), Value::Boolean(true));
        let keys = [SettingKey {
            key: "gym".to_owned(),
            default: Some(Value::Boolean(false)),
        }];
        Game::save_settings(&config, &game.slug, &current, &keys).unwrap();

        let saved = Game::load(&config, &game.slug).unwrap();
        assert_eq!(saved.name, "GTA");
        assert_eq!(saved.settings["other_splitter"].as_integer(), Some(3));
        assert_eq!(saved.settings["gym"].as_bool(), Some(true));
    }

    #[test]
    fn lists_games_by_name_with_their_auto_splitter_counts() {
        let (_dir, config) = config();
        let portal = Game::create(&config, "portal").unwrap();
        let gta = Game::create(&config, "GTA").unwrap();
        let mut splitters = Splitters::default();
        splitters
            .associate(Path::new("/a.wasm"), &gta.slug)
            .unwrap();
        splitters
            .associate(Path::new("/b.wasm"), &gta.slug)
            .unwrap();
        fs::write(config.dir().join("games/notes.txt"), "ignored").unwrap();

        let (games, errors) = Game::list(&config, &splitters);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            games,
            [
                GameSummary {
                    slug: gta.slug,
                    name: "GTA".to_owned(),
                    splitters: 2
                },
                GameSummary {
                    slug: portal.slug,
                    name: "portal".to_owned(),
                    splitters: 0
                },
            ]
        );
    }

    #[test]
    fn listing_reports_broken_files_and_keeps_the_rest() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        fs::write(config.dir().join("games/broken.toml"), "name = [").unwrap();
        let (games, errors) = Game::list(&config, &Splitters::default());
        assert_eq!(games.len(), 1);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("broken.toml"), "{errors:?}");
    }

    #[test]
    fn with_no_games_folder_there_are_no_games() {
        let (_dir, config) = config();
        let (games, errors) = Game::list(&config, &Splitters::default());
        assert!(games.is_empty() && errors.is_empty());
    }
}
