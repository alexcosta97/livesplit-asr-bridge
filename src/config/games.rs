//! `games/<game-slug>.toml`: a game's display name, its categories with
//! their saved auto splitter settings, and the category last used (spec
//! §7.1, §7.4).
//!
//! ```toml
//! name = "GTA San Andreas — Definitive Edition"
//! speedrun_id = "yo1yv1q5"
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

use crate::speedrun::{CategoryLink, SpeedrunLink};

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
    /// The game's id on speedrun.com, once linked (spec §7.5).
    pub speedrun_id: Option<String>,
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
    /// The category on speedrun.com this one is for, once linked (spec §7.5.1).
    pub speedrun: Option<CategoryLink>,
}

/// A game in the list the Game dialog offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSummary {
    pub slug: String,
    pub name: String,
    /// The game's id on speedrun.com, once linked.
    pub speedrun_id: Option<String>,
    /// How many known auto splitters are for this game.
    pub splitters: usize,
}

/// The contents of a game's file.
#[derive(Debug, Serialize, Deserialize)]
struct GameFile {
    name: String,
    /// The game's id on speedrun.com, once linked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speedrun_id: Option<String>,
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
    /// Missing in a hand-edited file: the key names the category then.
    #[serde(default)]
    name: Option<String>,
    /// The category's id on speedrun.com. The keys of its link are declared
    /// before `settings`, so they are written above that table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speedrun_id: Option<String>,
    /// The variables and values that pick the category's variant.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    speedrun_values: BTreeMap<String, String>,
    /// The level, for a category of a level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speedrun_level: Option<String>,
    #[serde(default)]
    settings: Table,
}

impl CategoryFile {
    /// The link the file's keys describe. Values or a level without an id are
    /// not a link.
    fn link(&self) -> Option<CategoryLink> {
        Some(CategoryLink {
            id: self.speedrun_id.clone()?,
            values: self.speedrun_values.clone(),
            level: self.speedrun_level.clone(),
        })
    }
}

impl Game {
    /// A game with only the category "Default", with no settings: a new
    /// game, or one whose file is missing or can't be read.
    pub fn empty(slug: &str, name: &str) -> Self {
        Self {
            slug: slug.to_owned(),
            name: name.to_owned(),
            speedrun_id: None,
            categories: vec![RunCategory {
                key: category_key(DEFAULT_CATEGORY),
                name: DEFAULT_CATEGORY.to_owned(),
                settings: Table::new(),
                speedrun: None,
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
            speedrun_id: file.speedrun_id,
            categories: file
                .categories
                .into_iter()
                .map(|(key, category)| RunCategory {
                    speedrun: category.link(),
                    name: category.name.unwrap_or_else(|| key.clone()),
                    key,
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
                speedrun: None,
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

    /// The category linked to `link`, if any.
    pub fn linked_to(&self, link: &CategoryLink) -> Option<&RunCategory> {
        self.categories
            .iter()
            .find(|category| category.speedrun.as_ref() == Some(link))
    }

    /// An error if another category than `except` (a key) already has `link`.
    fn check_link(&self, link: Option<&CategoryLink>, except: Option<&str>) -> Result<(), String> {
        let taken = link
            .and_then(|link| self.linked_to(link))
            .filter(|category| Some(category.key.as_str()) != except);
        match taken {
            Some(category) => Err(format!(
                "the category {} is already linked to that speedrun.com category",
                category.name
            )),
            None => Ok(()),
        }
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
                speedrun: None,
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
    /// without switching to it, linked to the speedrun.com category `link` if
    /// given. A link another category has is refused. Returns the game and
    /// the new category's key.
    pub fn add_category(
        config: &Config,
        slug: &str,
        name: &str,
        settings: &Table,
        link: Option<&CategoryLink>,
    ) -> Result<(Self, String), String> {
        let mut game = Self::load(config, slug)?;
        let name = game.check_name(name, None)?;
        game.check_link(link, None)?;
        let key = game.free_key(&name);
        game.categories.push(RunCategory {
            key: key.clone(),
            name,
            settings: settings.clone(),
            speedrun: link.cloned(),
        });
        game.sort();
        game.write(config)?;
        Ok((game, key))
    }

    /// Renames the category `key` of the game `slug` to `name`, and sets its
    /// link to `link`: `None` unlinks it. A link another category has is
    /// refused. Its key stays the same.
    pub fn rename_category(
        config: &Config,
        slug: &str,
        key: &str,
        name: &str,
        link: Option<&CategoryLink>,
    ) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        let name = game.check_name(name, Some(key))?;
        game.check_link(link, Some(key))?;
        let category = game.category_mut(key)?;
        category.name = name;
        category.speedrun = link.cloned();
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

    /// Links the game `slug` to speedrun.com, or unlinks it with `None`. A
    /// link also names the game as speedrun.com does; unlinking keeps the
    /// name. The slug, and so the file, stay the same. Another id, or none,
    /// unlinks every category from speedrun.com. The file is read again
    /// first.
    pub fn set_link(
        config: &Config,
        slug: &str,
        link: Option<&SpeedrunLink>,
    ) -> Result<Self, String> {
        let mut game = Self::load(config, slug)?;
        let id = link.map(|link| link.id.clone());
        if id != game.speedrun_id {
            // The categories' links are ids within the old game.
            for category in &mut game.categories {
                category.speedrun = None;
            }
        }
        game.speedrun_id = id;
        if let Some(link) = link {
            game.name = link.name.clone();
        }
        game.write(config)?;
        Ok(game)
    }

    /// Writes the game's file in the current form, so nothing is left to
    /// name.
    fn write(&mut self, config: &Config) -> Result<(), String> {
        let file = GameFile {
            name: self.name.clone(),
            speedrun_id: self.speedrun_id.clone(),
            category: Some(self.current.clone()),
            settings: None,
            categories: self
                .categories
                .iter()
                .map(|category| {
                    (
                        category.key.clone(),
                        CategoryFile {
                            name: Some(category.name.clone()),
                            speedrun_id: category.speedrun.as_ref().map(|link| link.id.clone()),
                            speedrun_values: category
                                .speedrun
                                .as_ref()
                                .map(|link| link.values.clone())
                                .unwrap_or_default(),
                            speedrun_level: category
                                .speedrun
                                .as_ref()
                                .and_then(|link| link.level.clone()),
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
                    speedrun_id: game.speedrun_id,
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
    fn a_category_without_a_name_is_named_after_its_key() {
        let (_dir, config) = config();
        fs::create_dir_all(config.dir().join("games")).unwrap();
        fs::write(
            config.game_file("gta"),
            "name = \"GTA\"\n\n[categories.any]\n\n[categories.any.settings]\ngym = true\n",
        )
        .unwrap();
        let mut game = Game::load(&config, "gta").unwrap();
        assert_eq!(game.categories.len(), 1);
        assert_eq!(game.categories[0].key, "any");
        assert_eq!(game.categories[0].name, "any");
        // Writing always writes a name.
        game.write(&config).unwrap();
        let text = fs::read_to_string(config.game_file("gta")).unwrap();
        assert!(text.contains("name = \"any\""));
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
                    speedrun_id: None,
                    splitters: 2
                },
                GameSummary {
                    slug: portal.slug,
                    name: "portal".to_owned(),
                    speedrun_id: None,
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
        let game = Game::rename_category(&config, "gta", "default", "Any%", None).unwrap();
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
            Game::add_category(&config, &game.slug, " 100% ", &table("gym = true"), None).unwrap();
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
        let (_, any) = Game::add_category(&config, "gta", "Any%", &Table::new(), None).unwrap();
        let (_, other) = Game::add_category(&config, "gta", "Any", &Table::new(), None).unwrap();
        assert_eq!((any.as_str(), other.as_str()), ("any", "any-2"));
        // Renaming changes only the name.
        let game = Game::rename_category(&config, "gta", "any", "Any% NMG", None).unwrap();
        let renamed = game.categories.iter().find(|c| c.key == "any").unwrap();
        assert_eq!(renamed.name, "Any% NMG");
    }

    #[test]
    fn a_name_already_used_in_any_case_or_blank_is_refused() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "Any%", &Table::new(), None).unwrap();
        assert!(Game::add_category(&config, "gta", "any%", &Table::new(), None).is_err());
        assert!(Game::add_category(&config, "gta", "  ", &Table::new(), None).is_err());
        assert!(Game::rename_category(&config, "gta", "default", "ANY%", None).is_err());
        // A category keeps its own name, in another case.
        assert!(Game::rename_category(&config, "gta", "any", "ANY%", None).is_ok());
    }

    #[test]
    fn the_category_in_use_and_the_last_one_cannot_be_deleted() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        assert!(Game::delete_category(&config, "gta", "default").is_err());
        Game::add_category(&config, "gta", "100%", &Table::new(), None).unwrap();
        assert!(Game::delete_category(&config, "gta", "default").is_err());
        let game = Game::delete_category(&config, "gta", "100").unwrap();
        assert_eq!(names(&game), ["Default"]);
        assert_eq!(names(&Game::load(&config, "gta").unwrap()), ["Default"]);
    }

    #[test]
    fn the_category_used_is_remembered() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "100%", &table("gym = true"), None).unwrap();
        let game = Game::use_category(&config, "gta", "100").unwrap();
        assert_eq!(game.settings(), &table("gym = true"));
        assert_eq!(Game::load(&config, "gta").unwrap().current, "100");
        assert!(Game::use_category(&config, "gta", "gone").is_err());
    }

    #[test]
    fn saving_into_one_category_leaves_the_others_alone() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "100%", &table("gym = false"), None).unwrap();
        Game::save_settings(&config, "gta", "default", &table("gym = true")).unwrap();
        let game = Game::load(&config, "gta").unwrap();
        let other = game.categories.iter().find(|c| c.key == "100").unwrap();
        assert_eq!(other.settings, table("gym = false"));
        assert_eq!(game.settings(), &table("gym = true"));
    }

    #[test]
    fn a_linked_game_keeps_its_speedrun_id() {
        let (_dir, config) = config();
        fs::create_dir_all(config.games_dir()).unwrap();
        let path = config.game_file("portal");
        fs::write(&path, "name = \"Portal\"\nspeedrun_id = \"4pd0n31e\"\n").unwrap();
        let game = Game::load(&config, "portal").unwrap();
        assert_eq!(game.speedrun_id.as_deref(), Some("4pd0n31e"));

        Game::save_settings(&config, "portal", &game.current, &table("gym = true")).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("name = \"Portal\"\nspeedrun_id = \"4pd0n31e\"\n"),
            "{text}"
        );
    }

    #[test]
    fn a_game_without_a_link_writes_no_speedrun_id() {
        let (_dir, config) = config();
        Game::create(&config, "Portal").unwrap();
        let text = fs::read_to_string(config.game_file("portal")).unwrap();
        assert!(!text.contains("speedrun_id"), "{text}");
    }

    #[test]
    fn linking_names_the_game_as_on_speedrun_com_and_keeps_its_slug() {
        let (_dir, config) = config();
        Game::create(&config, "GTA SA DE").unwrap();
        let link = SpeedrunLink {
            id: "yo1yv1q5".into(),
            name: "Grand Theft Auto: San Andreas".into(),
        };
        let game = Game::set_link(&config, "gta-sa-de", Some(&link)).unwrap();
        assert_eq!(
            (
                game.slug.as_str(),
                game.name.as_str(),
                game.speedrun_id.as_deref()
            ),
            (
                "gta-sa-de",
                "Grand Theft Auto: San Andreas",
                Some("yo1yv1q5")
            )
        );
        assert_eq!(Game::load(&config, "gta-sa-de").unwrap(), game);
    }

    #[test]
    fn unlinking_removes_the_id_and_keeps_the_name() {
        let (_dir, config) = config();
        Game::create(&config, "Portal").unwrap();
        let link = SpeedrunLink {
            id: "4pd0n31e".into(),
            name: "Portal (2007)".into(),
        };
        Game::set_link(&config, "portal", Some(&link)).unwrap();
        let game = Game::set_link(&config, "portal", None).unwrap();
        assert_eq!(game.speedrun_id, None);
        assert_eq!(game.name, "Portal (2007)");
        let text = fs::read_to_string(config.game_file("portal")).unwrap();
        assert!(!text.contains("speedrun_id"), "{text}");
    }

    #[test]
    fn the_list_tells_which_games_are_linked() {
        let (_dir, config) = config();
        Game::create(&config, "Portal").unwrap();
        Game::create(&config, "Doom").unwrap();
        let link = SpeedrunLink {
            id: "4pd0n31e".into(),
            name: "Portal".into(),
        };
        Game::set_link(&config, "portal", Some(&link)).unwrap();
        let (games, errors) = Game::list(&config, &Splitters::default());
        assert!(errors.is_empty());
        let ids: Vec<_> = games
            .iter()
            .map(|g| (g.slug.as_str(), g.speedrun_id.as_deref()))
            .collect();
        assert_eq!(ids, [("doom", None), ("portal", Some("4pd0n31e"))]);
    }

    fn link(id: &str, values: &[(&str, &str)], level: Option<&str>) -> CategoryLink {
        CategoryLink {
            id: id.to_owned(),
            values: values
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            level: level.map(str::to_owned),
        }
    }

    fn pc() -> CategoryLink {
        link("jdzg83rk", &[("yn2wk42n", "21gzep8l")], None)
    }

    /// The `[categories.<key>]` table of a game's file.
    fn category_table(config: &Config, slug: &str, key: &str) -> Table {
        let text = fs::read_to_string(config.game_file(slug)).unwrap();
        let file: Table = toml::from_str(&text).unwrap();
        file["categories"][key].as_table().unwrap().clone()
    }

    #[test]
    fn a_category_link_is_written_and_read_back() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        let (_, key) =
            Game::add_category(&config, "gta", "Any% (PC)", &Table::new(), Some(&pc())).unwrap();
        let written = category_table(&config, "gta", &key);
        assert_eq!(written["speedrun_id"].as_str(), Some("jdzg83rk"));
        assert_eq!(
            written["speedrun_values"]["yn2wk42n"].as_str(),
            Some("21gzep8l")
        );
        assert!(!written.contains_key("speedrun_level"));
        // The keys come above the settings table.
        let text = fs::read_to_string(config.game_file("gta")).unwrap();
        assert!(text.find("speedrun_id").unwrap() < text.find("settings").unwrap());
        let game = Game::load(&config, "gta").unwrap();
        assert_eq!(game.linked_to(&pc()).unwrap().key, key);
    }

    #[test]
    fn an_unlinked_category_writes_no_speedrun_keys() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        let written = category_table(&config, "gta", "default");
        assert!(written.keys().all(|key| !key.starts_with("speedrun")));
        assert_eq!(
            Game::load(&config, "gta").unwrap().categories[0].speedrun,
            None
        );
    }

    #[test]
    fn a_level_link_keeps_its_level() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        let level = link("abc", &[], Some("lvl1"));
        let (game, key) =
            Game::add_category(&config, "gta", "Level", &Table::new(), Some(&level)).unwrap();
        let written = category_table(&config, "gta", &key);
        assert_eq!(written["speedrun_level"].as_str(), Some("lvl1"));
        assert!(!written.contains_key("speedrun_values"));
        assert_eq!(game.linked_to(&level).unwrap().key, key);
        assert_eq!(Game::load(&config, "gta").unwrap(), game);
    }

    #[test]
    fn renaming_without_a_link_unlinks_and_keeps_the_name() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        let (_, key) =
            Game::add_category(&config, "gta", "Any%", &Table::new(), Some(&pc())).unwrap();
        // Renaming with its own link is fine.
        let game = Game::rename_category(&config, "gta", &key, "Any% PC", Some(&pc())).unwrap();
        assert!(game.linked_to(&pc()).is_some());
        let game = Game::rename_category(&config, "gta", &key, "Any% PC", None).unwrap();
        assert!(game.linked_to(&pc()).is_none());
        assert!(names(&game).contains(&"Any% PC"));
        assert!(!category_table(&config, "gta", &key).contains_key("speedrun_id"));
    }

    #[test]
    fn a_link_another_category_has_is_refused() {
        let (_dir, config) = config();
        Game::create(&config, "GTA").unwrap();
        Game::add_category(&config, "gta", "Any%", &Table::new(), Some(&pc())).unwrap();
        assert!(Game::add_category(&config, "gta", "Other", &Table::new(), Some(&pc())).is_err());
        assert!(Game::rename_category(&config, "gta", "default", "Default", Some(&pc())).is_err());
    }

    #[test]
    fn changing_the_game_s_link_unlinks_its_categories() {
        let (_dir, config) = config();
        let game = |id: &str| SpeedrunLink {
            id: id.to_owned(),
            name: "GTA".to_owned(),
        };
        Game::create(&config, "GTA").unwrap();
        Game::set_link(&config, "gta", Some(&game("g1"))).unwrap();
        let relink = || {
            let game = Game::rename_category(&config, "gta", "any", "Any%", Some(&pc())).unwrap();
            assert!(game.linked_to(&pc()).is_some());
        };
        Game::add_category(&config, "gta", "Any%", &Table::new(), Some(&pc())).unwrap();
        let same = Game::set_link(&config, "gta", Some(&game("g1"))).unwrap();
        assert!(same.linked_to(&pc()).is_some());
        let other = Game::set_link(&config, "gta", Some(&game("g2"))).unwrap();
        assert!(other.categories.iter().all(|c| c.speedrun.is_none()));
        assert_eq!(names(&other), ["Any%", "Default"]);
        relink();
        let none = Game::set_link(&config, "gta", None).unwrap();
        assert!(none.categories.iter().all(|c| c.speedrun.is_none()));
    }

    #[test]
    fn values_without_an_id_read_as_not_linked() {
        let (_dir, config) = config();
        fs::create_dir_all(config.game_file("gta").parent().unwrap()).unwrap();
        fs::write(
            config.game_file("gta"),
            "name = \"GTA\"\n[categories.any]\nname = \"Any%\"\nspeedrun_values = { a = \"b\" }\nspeedrun_level = \"l\"\n",
        )
        .unwrap();
        let game = Game::load(&config, "gta").unwrap();
        assert_eq!(game.categories[0].speedrun, None);
    }
}
