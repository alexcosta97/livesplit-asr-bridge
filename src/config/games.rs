//! `games/<game-slug>.toml`: a game's display name, its categories with
//! their saved auto splitter settings, and the category last used (spec
//! §7.1, §7.4).
//!
//! ```toml
//! name = "GTA San Andreas — Definitive Edition"
//! category = "any"
//!
//! [categories.any]
//! name = "Any%"
//!
//! [categories.any.settings]
//! start_on_new_game = true
//! ```

use std::{collections::BTreeMap, fs};

use serde::{Deserialize, Serialize};
use toml::Table;

use super::{Config, Splitters, category_key, merge_saved, read_if_exists, slug, write_replacing};

/// The category a new game starts with, and the name a file from before
/// categories has its settings read as.
pub const DEFAULT_CATEGORY: &str = "Default";

/// A game, its categories and the category in use.
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    /// Names the game's file.
    pub slug: String,
    /// The name shown to people.
    pub name: String,
    /// The game's categories, by name. Never empty.
    pub categories: Vec<RunCategory>,
    /// The key of the category in use, always one of `categories`.
    pub current: String,
    /// The key of the category holding the settings of a file from before
    /// categories, until the file is written in the new form. The app asks
    /// which category they are for (spec §7.4).
    pub unnamed: Option<String>,
}

/// A category of a game, with its own saved settings.
#[derive(Debug, Clone, PartialEq)]
pub struct RunCategory {
    /// Names the category in the game's file. Made from its name when it was
    /// added, and kept when it is renamed.
    pub key: String,
    /// The name shown to people.
    pub name: String,
    /// The saved settings, shared by every auto splitter for the game while
    /// this category is in use.
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
    /// The key of the category last used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    /// The one settings map of a file from before categories. Read, never
    /// written.
    #[serde(default, skip_serializing)]
    settings: Option<Table>,
    #[serde(default)]
    categories: BTreeMap<String, CategoryFile>,
}

/// A category in a game's file.
#[derive(Debug, Serialize, Deserialize)]
struct CategoryFile {
    name: String,
    #[serde(default)]
    settings: Table,
}

impl Game {
    /// A game with only the category "Default", with no settings: a new
    /// game, or one whose file is missing or can't be read.
    pub fn empty(slug: &str, name: &str) -> Self {
        Self {
            slug: slug.to_owned(),
            name: name.to_owned(),
            categories: vec![RunCategory {
                key: category_key(DEFAULT_CATEGORY),
                name: DEFAULT_CATEGORY.to_owned(),
                settings: Table::new(),
            }],
            current: category_key(DEFAULT_CATEGORY),
            unnamed: None,
        }
    }

    /// The category in use.
    pub fn category(&self) -> &RunCategory {
        self.categories
            .iter()
            .find(|category| category.key == self.current)
            .expect("the category in use is one of the game's")
    }

    /// The saved settings of the category in use.
    pub fn settings(&self) -> &Table {
        &self.category().settings
    }

    /// Whether another category than `except` (a key) is called `name`,
    /// ignoring case and surrounding spaces.
    pub fn name_taken(&self, name: &str, except: Option<&str>) -> bool {
        let name = name.trim().to_lowercase();
        self.categories.iter().any(|category| {
            Some(category.key.as_str()) != except && category.name.trim().to_lowercase() == name
        })
    }

    /// Reads the game `slug`'s file. A missing file, for example one deleted
    /// by hand, is a game with that slug as its name and only the category
    /// "Default".
    pub fn load(config: &Config, slug: &str) -> Result<Self, String> {
        let path = config.game_file(slug);
        let Some(text) = read_if_exists(&path)? else {
            return Ok(Self::empty(slug, slug));
        };
        let file: GameFile = toml::from_str(&text)
            .map_err(|error| format!("couldn't read {}: {error}", path.display()))?;
        Ok(Self::from_file(slug, file))
    }

    /// The game a file describes. A file from before categories has its
    /// settings read as the category "Default" (spec §7.4).
    fn from_file(slug: &str, file: GameFile) -> Self {
        let mut game = Self {
            slug: slug.to_owned(),
            name: file.name,
            categories: file
                .categories
                .into_iter()
                .map(|(key, category)| RunCategory {
                    key,
                    name: category.name,
                    settings: category.settings,
                })
                .collect(),
            current: String::new(),
            unnamed: None,
        };
        let mut current = file.category;
        if let Some(settings) = file.settings {
            let name = game.free_name(DEFAULT_CATEGORY);
            let key = game.free_key(&name);
            current.get_or_insert_with(|| key.clone());
            game.unnamed = Some(key.clone());
            game.categories.push(RunCategory {
                key,
                name,
                settings,
            });
        }
        if game.categories.is_empty() {
            game.categories = Self::empty(slug, "").categories;
        }
        game.sort();
        game.current = current
            .filter(|key| game.categories.iter().any(|category| &category.key == key))
            .unwrap_or_else(|| game.categories[0].key.clone());
        game
    }

    /// `base`, or `base 2`, `base 3`… if a category already has that name.
    fn free_name(&self, base: &str) -> String {
        (1..)
            .map(|n| match n {
                1 => base.to_owned(),
                n => format!("{base} {n}"),
            })
            .find(|name| !self.name_taken(name, None))
            .expect("an unused name")
    }

    /// The key for a new category called `name`, with `-2`, `-3`… if
    /// another category has it.
    fn free_key(&self, name: &str) -> String {
        let base = category_key(name);
        (1..)
            .map(|n| match n {
                1 => base.clone(),
                n => format!("{base}-{n}"),
            })
            .find(|key| self.categories.iter().all(|category| &category.key != key))
            .expect("an unused key")
    }

    /// Keeps the categories by name, ignoring case.
    fn sort(&mut self) {
        self.categories
            .sort_by_cached_key(|category| (category.name.to_lowercase(), category.key.clone()));
    }

    /// The category `key`, or an error naming it if the game has none.
    fn category_mut(&mut self, key: &str) -> Result<&mut RunCategory, String> {
        self.categories
            .iter_mut()
            .find(|category| category.key == key)
            .ok_or_else(|| format!("there is no category {key}"))
    }

    /// `name` trimmed, or an error if it is blank or another category than
    /// `except` has it.
    fn check_name(&self, name: &str, except: Option<&str>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("a category needs a name".to_owned());
        }
        if self.name_taken(name, except) {
            return Err(format!("there is already a category called {name}"));
        }
        Ok(name.to_owned())
    }

    /// Sets up a new game called `name`, with only the category "Default",
    /// and writes its file. Its slug is made from the name, with a number
    /// added if another game already has that slug.
    pub fn create(config: &Config, name: &str) -> Result<Self, String> {
        let base = slug(name);
        let slug = (1..)
            .map(|n| match n {
                1 => base.clone(),
                n => format!("{base}-{n}"),
            })
            .find(|slug| !config.game_file(slug).exists())
            .expect("an unused slug");
        let mut game = Self::empty(&slug, name.trim());
        game.write(config)?;
        Ok(game)
    }

    /// Saves `current`, the running auto splitter's whole settings map, into
    /// the category `key` with the rules of spec §7.2, and records it as the
    /// category in use. The file is read again first, so its names, other
    /// categories and dates are the latest. A category deleted by hand
    /// meanwhile is added back, named after its key.
    pub fn save_settings(
        config: &Config,
        slug: &str,
        key: &str,
        current: &Table,
    ) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        if game.category_mut(key).is_err() {
            game.categories.push(RunCategory {
                key: key.to_owned(),
                name: game.free_name(key),
                settings: Table::new(),
            });
            game.sort();
        }
        let category = game.category_mut(key)?;
        category.settings = merge_saved(&category.settings, current);
        game.current = key.to_owned();
        game.write(config)?;
        Ok(game)
    }

    /// Adds a category called `name` with `settings` to the game `slug`,
    /// without switching to it. Returns the game and the new category's key.
    pub fn add_category(
        config: &Config,
        slug: &str,
        name: &str,
        settings: &Table,
    ) -> Result<(Self, String), String> {
        let mut game = Self::load(config, slug)?;
        let name = game.check_name(name, None)?;
        let key = game.free_key(&name);
        game.categories.push(RunCategory {
            key: key.clone(),
            name,
            settings: settings.clone(),
        });
        game.sort();
        game.write(config)?;
        Ok((game, key))
    }

    /// Renames the category `key` of the game `slug` to `name`. Its key
    /// stays the same.
    pub fn rename_category(
        config: &Config,
        slug: &str,
        key: &str,
        name: &str,
    ) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        let name = game.check_name(name, Some(key))?;
        game.category_mut(key)?.name = name;
        game.sort();
        game.write(config)?;
        Ok(game)
    }

    /// Deletes the category `key` of the game `slug` and its settings. The
    /// category in use and the last category can't be deleted.
    pub fn delete_category(config: &Config, slug: &str, key: &str) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        if game.current == key {
            return Err("the category in use can't be deleted".to_owned());
        }
        if game.categories.len() == 1 {
            return Err("a game's last category can't be deleted".to_owned());
        }
        game.category_mut(key)?;
        game.categories.retain(|category| category.key != key);
        game.write(config)?;
        Ok(game)
    }

    /// Records the category `key` as the one in use for the game `slug`.
    pub fn use_category(config: &Config, slug: &str, key: &str) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        game.category_mut(key)?;
        game.current = key.to_owned();
        game.write(config)?;
        Ok(game)
    }

    /// Writes the game's file in the current form, so nothing is left to
    /// name.
    fn write(&mut self, config: &Config) -> Result<(), String> {
        let file = GameFile {
            name: self.name.clone(),
            category: Some(self.current.clone()),
            settings: None,
            categories: self
                .categories
                .iter()
                .map(|category| {
                    (
                        category.key.clone(),
                        CategoryFile {
                            name: category.name.clone(),
                            settings: category.settings.clone(),
                        },
                    )
                })
                .collect(),
        };
        let text = toml::to_string(&file).map_err(|error| error.to_string())?;
        write_replacing(&config.game_file(&self.slug), &text)?;
        self.unnamed = None;
        Ok(())
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
        assert_eq!(
            text,
            "name = \"GTA San Andreas — DE\"\ncategory = \"default\"\n\n[categories.default]\nname = \"Default\"\n\n[categories.default.settings]\n"
        );
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
        assert!(game.settings().is_empty());
        assert_eq!(game.category().name, "Default");
    }

    #[test]
    fn saving_settings_writes_the_whole_map_and_keeps_the_name() {
        let (_dir, config) = config();
        let game = Game::create(&config, "GTA").unwrap();
        let mut current = Table::new();
        current.insert("other_splitter".to_owned(), Value::Integer(3));
        current.insert("gym".to_owned(), Value::Boolean(true));
        Game::save_settings(&config, &game.slug, &game.current, &current).unwrap();

        let saved = Game::load(&config, &game.slug).unwrap();
        assert_eq!(saved.name, "GTA");
        assert_eq!(saved.settings(), &current);
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

    fn table(text: &str) -> Table {
        text.parse().unwrap()
    }

    fn names(game: &Game) -> Vec<&str> {
        game.categories
            .iter()
            .map(|category| category.name.as_str())
            .collect()
    }

    #[test]
    fn a_file_from_before_categories_is_read_as_default() {
        let (_dir, config) = config();
        write_replacing(
            &config.game_file("gta"),
            "name = \"GTA\"\n\n[settings]\ngym = true\n",
        )
        .unwrap();
        let game = Game::load(&config, "gta").unwrap();
        assert_eq!(names(&game), ["Default"]);
        assert_eq!(game.current, "default");
        assert_eq!(game.settings(), &table("gym = true"));
        assert_eq!(game.unnamed.as_deref(), Some("default"));
    }

    #[test]
    fn naming_the_old_category_keeps_its_key_and_ends_the_old_form() {
        let (_dir, config) = config();
        write_replacing(
            &config.game_file("gta"),
            "name = \"GTA\"\n\n[settings]\ngym = true\n",
        )
        .unwrap();
        let game = Game::rename_category(&config, "gta", "default", "Any%").unwrap();
        assert_eq!(names(&game), ["Any%"]);
        assert_eq!(game.current, "default");
        assert_eq!(game.unnamed, None);
        let saved = Game::load(&config, "gta").unwrap();
        assert_eq!(saved.unnamed, None);
        assert_eq!(saved.settings(), &table("gym = true"));
    }

    #[test]
    fn a_file_with_only_categories_has_nothing_to_name() {
        let (_dir, config) = config();
        let game = Game::create(&config, "GTA").unwrap();
        assert_eq!(game.unnamed, None);
        assert_eq!(Game::load(&config, "gta").unwrap().unnamed, None);
    }

    #[test]
    fn an_old_map_beside_categories_becomes_another_category() {
        let (_dir, config) = config();
        write_replacing(
            &config.game_file("gta"),
            "name = \"GTA\"\ncategory = \"any\"\n\n[settings]\ngym = true\n\n\
             [categories.any]\nname = \"Any%\"\n\n\
             [categories.default]\nname = \"Default\"\n",
        )
        .unwrap();
        let game = Game::load(&config, "gta").unwrap();
        assert_eq!(names(&game), ["Any%", "Default", "Default 2"]);
        assert_eq!(game.current, "any");
        let old = game
            .categories
            .iter()
            .find(|c| c.name == "Default 2")
            .unwrap();
        assert_eq!(old.key, "default-2");
        assert_eq!(old.settings, table("gym = true"));
        assert_eq!(game.unnamed.as_deref(), Some("default-2"));
    }

    #[test]
    fn the_old_map_is_written_back_as_a_category() {
        let (_dir, config) = config();
        write_replacing(
            &config.game_file("gta"),
            "name = \"GTA\"\n\n[settings]\ngym = true\n",
        )
        .unwrap();
        Game::save_settings(&config, "gta", "default", &table("gym = true")).unwrap();
        let text = fs::read_to_string(config.game_file("gta")).unwrap();
        assert_eq!(
            text,
            "name = \"GTA\"\ncategory = \"default\"\n\n[categories.default]\nname = \"Default\"\n\n\
             [categories.default.settings]\ngym = true\n"
        );
    }

    #[test]
    fn a_category_in_use_that_has_gone_falls_back_to_the_first_by_name() {
        let (_dir, config) = config();
        write_replacing(
            &config.game_file("gta"),
            "name = \"GTA\"\ncategory = \"gone\"\n\n\
             [categories.b]\nname = \"b\"\n\n[categories.a]\nname = \"A\"\n",
        )
        .unwrap();
        let game = Game::load(&config, "gta").unwrap();
        assert_eq!(names(&game), ["A", "b"]);
        assert_eq!(game.current, "a");
    }

    #[test]
    fn a_new_category_copies_the_settings_it_is_given() {
        let (_dir, config) = config();
        let game = Game::create(&config, "GTA").unwrap();
        let (game, key) =
            Game::add_category(&config, &game.slug, " 100% ", &table("gym = true")).unwrap();
        assert_eq!(key, "100");
        assert_eq!(names(&game), ["100%", "Default"]);
        // Adding doesn't switch.
        assert_eq!(game.current, "default");
        let saved = Game::load(&config, "gta").unwrap();
        let added = saved.categories.iter().find(|c| c.key == "100").unwrap();
        assert_eq!(added.settings, table("gym = true"));
    }

    #[test]
    fn categories_whose_names_slug_alike_get_their_own_keys() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        let (_, any) = Game::add_category(&config, "gta", "Any%", &Table::new()).unwrap();
        let (_, other) = Game::add_category(&config, "gta", "Any", &Table::new()).unwrap();
        assert_eq!((any.as_str(), other.as_str()), ("any", "any-2"));
        // Renaming changes only the name.
        let game = Game::rename_category(&config, "gta", "any", "Any% NMG").unwrap();
        let renamed = game.categories.iter().find(|c| c.key == "any").unwrap();
        assert_eq!(renamed.name, "Any% NMG");
    }

    #[test]
    fn a_name_already_used_in_any_case_or_blank_is_refused() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "Any%", &Table::new()).unwrap();
        assert!(Game::add_category(&config, "gta", "any%", &Table::new()).is_err());
        assert!(Game::add_category(&config, "gta", "  ", &Table::new()).is_err());
        assert!(Game::rename_category(&config, "gta", "default", "ANY%").is_err());
        // A category keeps its own name, in another case.
        assert!(Game::rename_category(&config, "gta", "any", "ANY%").is_ok());
    }

    #[test]
    fn the_category_in_use_and_the_last_one_cannot_be_deleted() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        assert!(Game::delete_category(&config, "gta", "default").is_err());
        Game::add_category(&config, "gta", "100%", &Table::new()).unwrap();
        assert!(Game::delete_category(&config, "gta", "default").is_err());
        let game = Game::delete_category(&config, "gta", "100").unwrap();
        assert_eq!(names(&game), ["Default"]);
        assert_eq!(names(&Game::load(&config, "gta").unwrap()), ["Default"]);
    }

    #[test]
    fn the_category_used_is_remembered() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "100%", &table("gym = true")).unwrap();
        let game = Game::use_category(&config, "gta", "100").unwrap();
        assert_eq!(game.settings(), &table("gym = true"));
        assert_eq!(Game::load(&config, "gta").unwrap().current, "100");
        assert!(Game::use_category(&config, "gta", "gone").is_err());
    }

    #[test]
    fn saving_into_one_category_leaves_the_others_alone() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "100%", &table("gym = false")).unwrap();
        Game::save_settings(&config, "gta", "default", &table("gym = true")).unwrap();
        let game = Game::load(&config, "gta").unwrap();
        let other = game.categories.iter().find(|c| c.key == "100").unwrap();
        assert_eq!(other.settings, table("gym = false"));
        assert_eq!(game.settings(), &table("gym = true"));
    }
}
