# Settings per Category Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each game has one or more categories, each with its own auto splitter settings map, picked from the Auto splitter card and managed in a new Category dialog (issue #81).

**Architecture:** Categories live inside the game's existing file (`games/<slug>.toml`), so `config::Game` gains a list of `RunCategory` and the key of the one in use. Older files are converted in memory when read, and the app asks which category their settings are for. The UI keeps today's flow for changing the game: switching category writes what the auto splitter stored to the old category, hands the runner the new map with `Runner::set_settings`, and resets the settings editor while keeping its widgets.

**Tech Stack:** Rust (stable), eframe/egui 0.36, `toml`/`serde`, `tempfile` in tests.

**Spec:** `docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md`. Sections 6.2, 6.3, 6.4, 6.8, 6.11, 7.1, 7.2, 7.4 (new), 8.1, 12, 15.2, 15.4. Superdesign components: `SplitterCard`, `SplitterStrip`, `UnsavedDialog`, `GameDialog`, `CategoryDialog` (ids in `.superdesign/components/components.json`; template at `https://p.superdesign.dev/draftcomponent/<id>`).

## Global Constraints

- Every game has at least one category. A new game, and a missing game file, start with one category named `Default` (key `default`) with no settings.
- Category keys use the game slug rules (`config::slug`), with `category` when nothing is left (not `game`), and no Windows reserved-name suffix. A key taken by another category of the game gets `-2`, `-3`…
- Category names are trimmed, non-empty, and unique within a game ignoring case.
- A file with a top-level `[settings]` table is read as a category `Default` (or `Default 2`, `Default 3`… if that name is taken) holding the map, made the category in use if the file records none. The top-level `settings` is never written back.
- A game loaded with settings from before categories (`Game::unnamed`) opens the Category dialog in its naming state: "Name this category". **Save** is a rename of that category (its key stays, so the running auto splitter is unaffected) and writes the file; **Not now** (also Escape) keeps "Default" and writes nothing. It is asked on loading an auto splitter (which covers starting the app) and on changing its game, and comes back only while the file is still in the old form.
- The category in use and a game's last category can't be deleted.
- A new category copies the running auto splitter's map as it is now (`SettingsEditor::live()`), never unsaved edits.
- Adding, renaming and deleting are written to the file straight away. Only switching asks about unsaved settings ("Changing the category").
- The §7.2 rules apply per category: saving and the stored-settings write go to the category in use and never touch the others.
- Copy (exact strings): `CATEGORY` (card label), `IN USE`, `Rename`, `Done`, `+ New category`, `Add`, `Starts as a copy of the settings in use now ({name}).`, `Delete {name} and its settings? This can't be undone.`, `Keep`, `Delete`, `Cancel`, `Use this category`, `Each category keeps its own settings. The category in use can't be deleted.`, `Changing the category`, `Settings are saved per game and category, so auto splitters for the same game share them.`, `NAME THIS CATEGORY` (title), `These settings were saved before the bridge kept settings per category. Which category are they set up for?`, `Category` (field label), `Any%` (placeholder), `You can add more categories later. Not now keeps the name "Default".`, `Not now`, `Save`
- Checks before every commit: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.
- Conventional Commits, signed. Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A file edited by hand that has both `[settings]` and `[categories]`.** Nothing should be lost: the old map becomes another `Default…` category. Covered by a test in Task 1.
2. **`category` in the file names a key that no longer exists** (deleted by hand). The app should fall back to the first category by name, not panic on `Game::category()`. Covered by a test in Task 1.
3. **Two categories whose names slug the same way** ("Any%" and "Any"). Each gets its own key (`any`, `any-2`), and renaming one never changes the other's key. Covered by a test in Task 1.
4. **A store reported by the auto splitter after the switch, for the old map.** It must be ignored (settings epoch) and never land in the new category. `Runner::set_settings` bumps the epoch; the Task 5 test checks the old category got the store made *before* the switch and the new category didn't.
5. **Escape while renaming or adding.** It should leave the edit and keep the dialog open, not close the dialog. `Modal::should_close` fires on Escape; Task 4 handles it, with a test of the state method.

---

### Task 1: Categories in the game's file

**Files:**
- Modify: `src/config/names.rs` (split `slug` into `slugify` + `slug`, add `category_key`)
- Modify: `src/config/games.rs` (whole file: model, file format, conversion, operations, tests)
- Modify: `src/config/mod.rs:32-37` (exports)
- Modify: `src/ui/mod.rs` (mechanical: `game.settings` → `game.settings()`, `save_settings` gets the key, fallback game)
- Modify: `src/ui/settings_integration.rs:46,137` (mechanical)

**Interfaces:**
- Produces (used by Tasks 4 and 5):
  - `config::category_key(name: &str) -> String`
  - `config::DEFAULT_CATEGORY: &str = "Default"`
  - `config::RunCategory { pub key: String, pub name: String, pub settings: toml::Table }`
  - `config::Game { pub slug, pub name, pub categories: Vec<RunCategory>, pub current: String, pub unnamed: Option<String> }` (`unnamed`: the key of the category read from a top-level `[settings]` table, cleared once the file is written), plus:
    - `Game::empty(slug: &str, name: &str) -> Game`
    - `Game::category(&self) -> &RunCategory`, `Game::settings(&self) -> &Table`
    - `Game::name_taken(&self, name: &str, except: Option<&str>) -> bool`
    - `Game::save_settings(config, slug, key: &str, current: &Table) -> Result<Game, String>`
    - `Game::add_category(config, slug, name: &str, settings: &Table) -> Result<(Game, String), String>` (the `String` is the new key)
    - `Game::rename_category(config, slug, key: &str, name: &str) -> Result<Game, String>`
    - `Game::delete_category(config, slug, key: &str) -> Result<Game, String>`
    - `Game::use_category(config, slug, key: &str) -> Result<Game, String>`

- [ ] **Step 1: Write the failing tests for `category_key`**

In `src/config/names.rs`'s `mod tests`, add:

```rust
#[test]
fn a_category_key_follows_the_slug_rules() {
    assert_eq!(category_key("Any%"), "any");
    assert_eq!(category_key("100%"), "100");
    assert_eq!(category_key(" All Missions "), "all-missions");
}

#[test]
fn a_category_key_has_its_own_fallback_and_no_reserved_names() {
    assert_eq!(category_key("%%%"), "category");
    // Keys aren't file names, so Windows' device names are fine.
    assert_eq!(category_key("con"), "con");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked config::names`
Expected: compile error, `cannot find function category_key`.

- [ ] **Step 3: Split `slug` and add `category_key`**

In `src/config/names.rs`, move the loop of `slug` into a private `slugify` that returns the bare result. `slug` keeps the game file rules on top of it:

```rust
/// A filename-safe slug of a game name, for its file in `games/`: lowercase
/// letters and digits, with a single `-` for each run of anything else.
/// Letters outside ASCII are kept, since every OS allows them in file names.
pub fn slug(name: &str) -> String {
    let mut slug = slugify(name);
    if slug.is_empty() {
        slug.push_str("game");
    } else if WINDOWS_RESERVED.contains(&slug.as_str()) {
        slug.push_str("-game");
    }
    slug
}

/// The key of a category named `name` in its game's file (spec §7.4): the
/// slug rules, with "category" when nothing is left. Keys aren't file
/// names, so Windows' reserved names are allowed.
pub fn category_key(name: &str) -> String {
    let key = slugify(name);
    if key.is_empty() { "category".to_owned() } else { key }
}

/// Lowercase letters and digits of `name`, with a single `-` for each run of
/// anything else, at most `MAX_SLUG_CHARS` long. Empty if `name` has no
/// letters or digits.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut chars = 0;
    let mut gap = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        if !c.is_alphanumeric() {
            gap = true;
            continue;
        }
        let dash = gap && !slug.is_empty();
        let added = if dash { 2 } else { 1 };
        if chars + added > MAX_SLUG_CHARS {
            break;
        }
        if dash {
            slug.push('-');
        }
        slug.push(c);
        chars += added;
        gap = false;
    }
    slug
}
```

Update the module doc's first line to: `//! Game and category names: the slug a game's file is named after, the key of a category, and the name the Game dialog suggests for a new auto splitter.` (wrapped at 78 columns).

- [ ] **Step 4: Run the names tests**

Run: `cargo test --locked config::names`
Expected: PASS (the existing slug tests are unchanged).

- [ ] **Step 5: Write the failing tests for categories**

Replace the `mod tests` of `src/config/games.rs` with the existing tests updated for the new API, plus the new ones below. The existing tests change like this:
- `a_new_game_gets_a_file_named_after_its_slug`: the expected text becomes `"name = \"GTA San Andreas — DE\"\ncategory = \"default\"\n\n[categories.default]\nname = \"Default\"\n\n[categories.default.settings]\n"`.
- `a_missing_file_is_a_game_with_no_settings`: assert `game.settings().is_empty()` and `game.category().name == "Default"`.
- `saving_settings_writes_the_whole_map_and_keeps_the_name`: call `Game::save_settings(&config, &game.slug, &game.current, &current)` and compare `saved.settings()`.

New tests (add `use toml::Value;` is already there):

```rust
fn table(text: &str) -> Table {
    text.parse().unwrap()
}

fn names(game: &Game) -> Vec<&str> {
    game.categories.iter().map(|category| category.name.as_str()).collect()
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
    let old = game.categories.iter().find(|c| c.name == "Default 2").unwrap();
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
```

`write_replacing` and `Config::game_file` are reachable from `games.rs` tests through `super::*` since `games` is a child of `config`; add `use super::super::write_replacing;` in the tests module if `super::*` doesn't bring it in (it is imported at the top of `games.rs` already).

- [ ] **Step 6: Run them to see them fail**

Run: `cargo test --locked config::games`
Expected: compile errors (`no method named settings`, `no function add_category`, …).

- [ ] **Step 7: Implement the categories model**

Replace everything above `#[cfg(test)]` in `src/config/games.rs` with:

```rust
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

use super::{
    Config, Splitters, category_key, merge_saved, read_if_exists, slug, write_replacing,
};

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
```

Then keep `Game::list` exactly as it is, and close the `impl`.

In `src/config/mod.rs`, export the new names:

```rust
pub use games::{DEFAULT_CATEGORY, Game, GameSummary, RunCategory};
pub use names::{category_key, name_from_file, slug};
```

- [ ] **Step 8: Update the callers so the crate compiles**

In `src/ui/mod.rs`:
- line ~367 (`RunnerEvent::Loaded`): `let saved = self.game.as_ref().map(|game| game.settings().clone());`
- `write_settings` (~448): `match Game::save_settings(&config, &game.slug, &game.current, self.settings.live()) {`
- `fn game` (~491): replace the fallback struct literal with `Game::empty(slug, slug)`, and its doc with "If it can't be read, the error is reported and the game has only the category "Default", with no saved settings."
- `load_with` (~538): `config::to_runtime(game.settings())`
- `game_dialog`, `GameDialogKind::Change` arm (~605, ~611): `game.settings()` in both places.
- `save_settings` (~1006): `Game::save_settings(&config, &game.slug, &game.current, &draft)`
- tests, `app_with_unsaved_edit` (~1412): `app.settings.reset(game.settings().clone());`
- tests, `saved_gym` (~1429): `.settings()` instead of `.settings`, i.e. `Game::load(config, "gta").unwrap().settings().get("gym")?.as_bool()`

In `src/ui/settings_integration.rs`:
- `Harness::new` (~46): `Game::save_settings(&config, &game.slug, &game.current, &saved).unwrap();`
- `Harness::saved` (~137): `Game::load(&self.config, "gta").unwrap().settings().clone()`

Search for any other use: `grep -rn 'save_settings(\|\.settings\b' src | grep -i game` must show only updated lines.

- [ ] **Step 9: Run every test**

Run: `cargo test --locked`
Expected: PASS, including the new `config::games` and `config::names` tests and the unchanged settings integration tests.

- [ ] **Step 10: Lint, format, commit**

```bash
cargo fmt && cargo clippy --all-targets --locked -- -D warnings
git add src/config src/ui/mod.rs src/ui/settings_integration.rs
git commit -m "feat(settings): keep categories with their own settings in the game's file"
```

---

### Task 2: "Changing the category" in the Unsaved dialog

**Files:**
- Modify: `src/ui/components/unsaved_dialog.rs`
- Modify: `src/ui/mod.rs` (`fn unsaved_dialog`, ~966)

**Interfaces:**
- Produces: `UnsavedTrigger::ChangeCategory`; `unsaved_dialog(ctx, trigger, changes, game: Option<&str>, category: Option<&str>) -> Option<UnsavedAction>`.

- [ ] **Step 1: Write the failing tests**

In `unsaved_dialog.rs` tests, change every `message(...)` call to pass a category as a fourth argument, and add:

```rust
#[test]
fn names_the_category_after_the_game() {
    assert_eq!(
        message(UnsavedTrigger::ChangeCategory, 3, Some("GTA San Andreas"), Some("Any%")),
        "You changed 3 settings for GTA San Andreas (Any%). Changing the category without \
         saving discards them."
    );
}

#[test]
fn changing_the_category_saves_with_save() {
    assert_eq!(save_label(UnsavedTrigger::ChangeCategory), "Save");
}
```

Existing calls pass `None` for the category (for example `message(UnsavedTrigger::Reload, 1, Some("GTA"), None)`) so their expected strings stay the same.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked unsaved_dialog`
Expected: compile errors (no `ChangeCategory`, wrong number of arguments).

- [ ] **Step 3: Implement**

- Add the variant, documented: `/// Switching to another category in the Category dialog.` `ChangeCategory,` between `ChangeGame` and `Close`.
- `unsaved_dialog` and `contents` take `category: Option<&str>` after `game` and pass it to `message`.
- `message`:

```rust
/// The text naming how many settings of `game`, in `category`, changed, and
/// that `trigger` discards them. Without a game, it names neither.
fn message(
    trigger: UnsavedTrigger,
    changes: usize,
    game: Option<&str>,
    category: Option<&str>,
) -> String {
    let settings = match changes {
        1 => "1 setting".to_owned(),
        n => format!("{n} settings"),
    };
    let game = match (game, category) {
        (Some(game), Some(category)) => format!(" for {game} ({category})"),
        (Some(game), None) => format!(" for {game}"),
        (None, _) => String::new(),
    };
    let action = action(trigger);
    format!("You changed {settings}{game}. {action} without saving discards them.")
}
```

- `action`: `UnsavedTrigger::ChangeCategory => "Changing the category",`
- `save_label`: add `UnsavedTrigger::ChangeCategory` to the `"Save"` arm.
- Module doc: "…when reloading, opening another file, changing the game or the category, or closing the app with unsaved settings (spec §6.8)."

In `src/ui/mod.rs` `fn unsaved_dialog`:

```rust
let game = self.game.as_ref().map(|game| game.name.as_str());
let category = self.game.as_ref().map(|game| game.category().name.as_str());
if let Some(action) = components::unsaved_dialog(
    &self.ctx,
    pending.trigger(),
    self.settings.changes(),
    game,
    category,
) {
```

- [ ] **Step 4: Run tests, lint, commit**

Run: `cargo test --locked && cargo clippy --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: PASS.

```bash
git add src/ui/components/unsaved_dialog.rs src/ui/mod.rs
git commit -m "feat(settings): name the category in the unsaved settings dialog"
```

---

### Task 3: The category on the Auto splitter card and the splitter strip

**Files:**
- Modify: `src/ui/components/splitter_card.rs`
- Modify: `src/ui/components/splitter_strip.rs`
- Modify: `src/ui/mod.rs` (`status_column`, ~645)

**Interfaces:**
- Produces: `SplitterAction::ChangeCategory`; `splitter_card(ui, loaded, game: Option<&str>, category: Option<&str>, picking)`; `splitter_strip(ui, loaded, category: Option<&str>, picking)`.
- Task 5 makes `SplitterAction::ChangeCategory` open the dialog; in this task it does nothing (`Some(SplitterAction::ChangeCategory) => {}`), so the crate compiles.

- [ ] **Step 1: Write the failing headless test**

In `src/ui/mod.rs` tests, next to `the_tab_strip_fits_the_smallest_wide_window_with_unsaved_settings`, add a test that draws the status column and finds the category. Use the existing `draw_app(app, width)` helper if it returns nothing; this test needs the accessibility labels, so draw it like this:

```rust
/// The labels on screen after drawing the whole window `width` wide.
fn labels(app: &mut BridgeApp, width: f32) -> Vec<String> {
    let ctx = app.ctx.clone();
    ctx.enable_accesskit();
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width, 800.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        let mut frame = eframe::Frame::_new_kittest();
        eframe::App::ui(app, ui, &mut frame);
    });
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .map(|update| update.nodes)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(_, node)| node.label().or(node.value()).map(str::to_owned))
        .collect()
}

#[test]
fn the_card_and_the_strip_show_the_category_in_use() {
    let (_dir, mut app) = app_with_unsaved_edit();
    app.runner.set_loaded_for_tests("gta.wasm".into());
    let wide = labels(&mut app, 1280.0);
    assert!(wide.iter().any(|label| label == "CATEGORY"), "{wide:?}");
    assert!(wide.iter().any(|label| label == "Default"), "{wide:?}");
    let compact = labels(&mut app, 420.0);
    assert!(compact.iter().any(|label| label == "Default"), "{compact:?}");
}
```

`runner.loaded_path()` is `None` in unit tests because nothing was loaded, so add a hook to `Runner` in `src/runner/mod.rs`, after `loaded_path`:

```rust
    /// Pretends the auto splitter at `path` is loaded, for UI tests.
    #[cfg(test)]
    pub fn set_loaded_for_tests(&self, path: PathBuf) {
        self.shared.control().path = Some(path);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test --locked the_card_and_the_strip_show_the_category_in_use`
Expected: FAIL, no `CATEGORY` label.

- [ ] **Step 3: Implement the card**

In `splitter_card.rs`:
- Add `ChangeCategory` to `SplitterAction`, documented `/// Pick which of the game's categories' settings to use.`; document `Change` as "Change the game the loaded auto splitter is for."
- Add `category: Option<&str>` after `game`, documented in the function doc: "`category` is the name of the category in use, shown under the game with its own Change."
- After the game's **Change** button, when `category` is `Some`:

```rust
let mut change_category = false;
if let Some(category) = category {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(
            RichText::new("CATEGORY")
                .font(theme::mono(11.0))
                .color(theme::TEXT_MUTED),
        );
        ui.add(
            Label::new(RichText::new(category).font(theme::body(13.0)).color(theme::TEXT))
                .truncate(),
        );
    });
    ui.add_space(8.0);
    change_category = ui
        .add(Button::secondary("Change").size(ButtonSize::Xs))
        .on_hover_text("Choose which category's settings to use")
        .clicked();
}
```

  Declare `let mut change_category = false;` beside `let mut change = false;` (outside the `match`) so the buttons row can read it, and return `Some(SplitterAction::ChangeCategory)` when it is set, checked right after `change`.

- [ ] **Step 4: Implement the strip**

In `splitter_strip.rs`, add `category: Option<&str>` after `loaded`. In the right-to-left layout, after the button and before the left-to-right child, add the category so it is never shortened, and the separator:

```rust
if let (Some(_), Some(category)) = (loaded, category) {
    ui.label(RichText::new(category).font(theme::mono(12.0)).color(theme::TEXT));
    ui.label(RichText::new("·").font(theme::mono(12.0)).color(theme::TEXT_MUTED));
}
```

Update its doc: "The Splitter strip: the loaded file and the category in use, with Reload, … The file name is shortened when space is short, never the category (spec §6.3)."

- [ ] **Step 5: Pass the category from the status column**

In `status_column`:

```rust
let category = self.game.as_ref().map(|game| game.category().name.clone());
…
Width::Wide => components::splitter_card(ui, loaded, game.as_deref(), category.as_deref(), picking),
Width::Compact => components::splitter_strip(ui, loaded, category.as_deref(), picking),
…
Some(SplitterAction::ChangeCategory) => {}
```

- [ ] **Step 6: Run tests, lint, commit**

Run: `cargo test --locked && cargo clippy --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: PASS, including `the_tab_strip_fits_*` (the card grew taller; the column scrolls, nothing else moves).

```bash
git add src/ui/components/splitter_card.rs src/ui/components/splitter_strip.rs src/ui/mod.rs src/runner/mod.rs
git commit -m "feat(settings): show the category on the auto splitter card and strip"
```

---

### Task 4: The Category dialog component

**Files:**
- Create: `src/ui/components/category_dialog.rs`
- Modify: `src/ui/components/mod.rs` (module and export)
- Modify: `src/ui/components/game_dialog.rs` (the note line under the list)

**Interfaces:**
- Consumes: `config::{Game, RunCategory}`, `Game::name_taken` (Task 1).
- Produces (used by Task 5):
  - `CategoryDialog::new(game: &Game) -> CategoryDialog`
  - `CategoryDialog::refresh(&mut self, game: &Game, select: Option<&str>)`: shows the game's categories again, selecting `select` (or keeping the selection if it still exists, else the one in use), back to the list.
  - `CategoryDialog::back_to_list(&mut self)`: leaves renaming, adding or confirming a delete. The naming state stays.
  - `CategoryDialog::naming(game: &Game, key: &str) -> CategoryDialog`: the "Name this category" state for the category `key`. Its **Save** returns `Rename { key, name }`, **Not now** and Escape return `Cancel`.
  - `CategoryDialog::is_naming(&self) -> bool`
  - `enum CategoryDialogAction { Use(String), Add(String), Rename { key: String, name: String }, Delete(String), Cancel }`
  - `category_dialog(ctx: &egui::Context, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction>`

- [ ] **Step 1: Write the failing state tests**

Create `src/ui/components/category_dialog.rs` with only the tests module first:

```rust
#[cfg(test)]
mod tests {
    use toml::Table;

    use super::*;
    use crate::config::RunCategory;

    fn game() -> Game {
        let category = |key: &str, name: &str| RunCategory {
            key: key.to_owned(),
            name: name.to_owned(),
            settings: Table::new(),
        };
        Game {
            slug: "gta".to_owned(),
            name: "GTA".to_owned(),
            categories: vec![category("100", "100%"), category("any", "Any%")],
            current: "any".to_owned(),
            unnamed: None,
        }
    }

    #[test]
    fn naming_asks_for_a_new_name_and_escape_is_not_now() {
        let mut dialog = CategoryDialog::naming(&game(), "any");
        assert!(dialog.is_naming());
        assert_eq!(
            dialog.mode,
            Mode::Naming {
                key: "any".to_owned(),
                name: String::new(),
                focus: true
            }
        );
        // The category being named may keep its own name.
        assert!(dialog.name_ok("ANY%", Some("any")));
        assert!(!dialog.name_ok("100%", Some("any")));
        dialog.back_to_list();
        assert!(dialog.is_naming());
        assert!(dialog.escape());
    }

    #[test]
    fn opens_on_the_category_in_use() {
        let dialog = CategoryDialog::new(&game());
        assert_eq!(dialog.selected, "any");
        assert!(!dialog.can_delete());
        assert_eq!(dialog.use_action(), CategoryDialogAction::Use("any".to_owned()));
    }

    #[test]
    fn only_another_category_than_the_one_in_use_can_be_deleted() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.selected = "100".to_owned();
        assert!(dialog.can_delete());
        let mut one = game();
        one.categories.retain(|category| category.key == "any");
        let dialog = CategoryDialog::new(&one);
        assert!(!dialog.can_delete());
    }

    #[test]
    fn a_new_name_must_be_new_and_not_blank() {
        let dialog = CategoryDialog::new(&game());
        assert!(dialog.name_ok("Low%", None));
        assert!(!dialog.name_ok(" any% ", None));
        assert!(!dialog.name_ok("  ", None));
        assert!(dialog.name_ok("ANY%", Some("any")));
    }

    #[test]
    fn renaming_starts_from_the_name() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_rename("100");
        assert_eq!(
            dialog.mode,
            Mode::Renaming {
                key: "100".to_owned(),
                name: "100%".to_owned(),
                focus: true
            }
        );
    }

    #[test]
    fn escape_leaves_an_edit_before_it_closes_the_dialog() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_adding();
        assert!(!dialog.escape());
        assert_eq!(dialog.mode, Mode::List);
        assert!(dialog.escape());
    }

    #[test]
    fn refreshing_selects_the_category_asked_for_or_keeps_one_that_exists() {
        let mut dialog = CategoryDialog::new(&game());
        dialog.start_adding();
        let mut more = game();
        more.categories.push(RunCategory {
            key: "low".to_owned(),
            name: "Low%".to_owned(),
            settings: Table::new(),
        });
        dialog.refresh(&more, Some("low"));
        assert_eq!(dialog.selected, "low");
        assert_eq!(dialog.mode, Mode::List);
        dialog.refresh(&game(), None);
        // "low" is gone: back to the category in use.
        assert_eq!(dialog.selected, "any");
    }
}
```

Add `mod category_dialog;` and `pub use category_dialog::{CategoryDialog, CategoryDialogAction, category_dialog};` to `src/ui/components/mod.rs` (alphabetical: before `mod checkbox;`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked category_dialog`
Expected: compile errors (no `CategoryDialog`).

- [ ] **Step 3: Implement the state and the drawing**

Put this above the tests in `category_dialog.rs`:

```rust
//! Category dialog: picks which of the game's categories' settings to use,
//! and adds, renames and deletes categories (spec §6.8, §7.4).

use eframe::egui::{
    self, Align, Frame, Id, Key, Label, Layout, Margin, Modal, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, TextEdit, Ui, UiBuilder, vec2,
};

use super::{
    BACKDROP,
    button::{Button, ButtonSize},
};
use crate::{config::Game, ui::theme};

/// The dialog's width inside its padding: the dialog is 480 px wide.
const WIDTH: f32 = 432.0;
/// The height of a row in the list, and of a name field.
const ROW_HEIGHT: f32 = 36.0;
/// The list scrolls beyond this height.
const LIST_HEIGHT: f32 = 240.0;

/// What was done in the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CategoryDialogAction {
    /// Use the category with this key.
    Use(String),
    /// Add a category with this name.
    Add(String),
    Rename { key: String, name: String },
    /// Delete the category with this key.
    Delete(String),
    Cancel,
}

/// What the dialog is doing besides showing the list.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    List,
    /// A row is a name field. `focus` until the field has been focused.
    Renaming { key: String, name: String, focus: bool },
    /// The new category's name field is shown.
    Adding { name: String, focus: bool },
    /// Asking whether to delete the selected category.
    ConfirmDelete,
    /// "Name this category": asking which category the settings from before
    /// categories, in the category `key`, are for. `focus` until the field
    /// has been focused.
    Naming { key: String, name: String, focus: bool },
}

/// The Category dialog's state.
#[derive(Debug, Clone)]
pub struct CategoryDialog {
    /// The game, as last read: its name, its categories and the one in use.
    game: Game,
    /// The key of the selected category.
    selected: String,
    mode: Mode,
}

impl CategoryDialog {
    /// The dialog for `game`, with the category in use selected.
    pub fn new(game: &Game) -> Self {
        Self {
            game: game.clone(),
            selected: game.current.clone(),
            mode: Mode::List,
        }
    }

    /// Shows `game` as it is now, after a category was added, renamed or
    /// deleted, and goes back to the list. Selects `select`, or keeps the
    /// selection if it still exists, or else the category in use.
    pub fn refresh(&mut self, game: &Game, select: Option<&str>) {
        let selected = select.unwrap_or(&self.selected).to_owned();
        self.game = game.clone();
        self.selected = if self.exists(&selected) {
            selected
        } else {
            game.current.clone()
        };
        self.mode = Mode::List;
    }

    /// "Name this category" for `game`'s category `key`, which holds the
    /// settings of a file from before categories (spec §6.8, §7.4).
    pub fn naming(game: &Game, key: &str) -> Self {
        Self {
            game: game.clone(),
            selected: key.to_owned(),
            mode: Mode::Naming {
                key: key.to_owned(),
                name: String::new(),
                focus: true,
            },
        }
    }

    /// Whether the dialog is asking for the name of settings from before
    /// categories.
    pub fn is_naming(&self) -> bool {
        matches!(self.mode, Mode::Naming { .. })
    }

    /// Leaves renaming, adding or confirming a delete. Naming has no list
    /// to go back to, so it stays.
    pub fn back_to_list(&mut self) {
        if !self.is_naming() {
            self.mode = Mode::List;
        }
    }

    fn exists(&self, key: &str) -> bool {
        self.game.categories.iter().any(|category| category.key == key)
    }

    fn name_of(&self, key: &str) -> &str {
        self.game
            .categories
            .iter()
            .find(|category| category.key == key)
            .map_or(key, |category| category.name.as_str())
    }

    /// Whether `name` can be given to a category: not blank, and no other
    /// category than `except` has it.
    fn name_ok(&self, name: &str, except: Option<&str>) -> bool {
        !name.trim().is_empty() && !self.game.name_taken(name, except)
    }

    /// The category in use and the last one can't be deleted.
    fn can_delete(&self) -> bool {
        self.selected != self.game.current && self.game.categories.len() > 1
    }

    fn use_action(&self) -> CategoryDialogAction {
        CategoryDialogAction::Use(self.selected.clone())
    }

    fn start_rename(&mut self, key: &str) {
        self.selected = key.to_owned();
        self.mode = Mode::Renaming {
            key: key.to_owned(),
            name: self.name_of(key).to_owned(),
            focus: true,
        };
    }

    fn start_adding(&mut self) {
        self.mode = Mode::Adding {
            name: String::new(),
            focus: true,
        };
    }

    /// Escape, or a click outside: leaves an edit if there is one, and
    /// returns whether the dialog should close. Naming closes: it is Not now.
    fn escape(&mut self) -> bool {
        if self.mode == Mode::List || self.is_naming() {
            return true;
        }
        self.mode = Mode::List;
        false
    }
}

/// Shows the Category dialog over the window. Escape and clicking outside it
/// leave a rename, a new name or a delete question first, then cancel it.
pub fn category_dialog(
    ctx: &egui::Context,
    dialog: &mut CategoryDialog,
) -> Option<CategoryDialogAction> {
    let frame = Frame::NONE
        .fill(theme::PANEL)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(24));
    let response = Modal::new(Id::new("category dialog"))
        .backdrop_color(BACKDROP)
        .frame(frame)
        .show(ctx, |ui| contents(ui, dialog));
    if response.should_close() {
        return dialog.escape().then_some(CategoryDialogAction::Cancel);
    }
    response.inner
}

fn contents(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    ui.set_width(WIDTH);
    ui.spacing_mut().item_spacing.y = 0.0;
    if dialog.is_naming() {
        return naming(ui, dialog);
    }
    ui.label(
        RichText::new("CATEGORY")
            .font(theme::display(16.0))
            .color(theme::TEXT),
    );
    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new(&dialog.game.name)
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
        )
        .truncate(),
    );

    ui.add_space(16.0);
    let mut action = list(ui, dialog);

    ui.add_space(12.0);
    action = action.or_else(|| adding(ui, dialog));
    if dialog.mode == Mode::ConfirmDelete {
        ui.add_space(12.0);
        action = action.or_else(|| confirm_delete(ui, dialog));
    }

    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(
                "Each category keeps its own settings. The category in use can't be deleted.",
            )
            .font(theme::body(12.0))
            .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );

    ui.add_space(20.0);
    let buttons = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let delete = ui.add(
                Button::secondary("Delete")
                    .enabled(dialog.can_delete() && dialog.mode == Mode::List),
            );
            if delete.clicked() {
                dialog.mode = Mode::ConfirmDelete;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let use_it = ui.add(Button::primary("Use this category"));
                let cancel = ui.add(Button::secondary("Cancel"));
                if use_it.clicked() {
                    Some(dialog.use_action())
                } else if cancel.clicked() {
                    Some(CategoryDialogAction::Cancel)
                } else {
                    None
                }
            })
            .inner
        })
        .inner;
    action.or(buttons)
}

/// The categories, each with Rename, or a name field for the one being
/// renamed.
fn list(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let mut action = None;
    let mut select = None;
    let mut rename = None;
    ScrollArea::vertical()
        .max_height(LIST_HEIGHT)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let categories = dialog.game.categories.clone();
            for category in &categories {
                if let Mode::Renaming { key, name, focus } = &mut dialog.mode
                    && *key == category.key
                {
                    let ok = !name.trim().is_empty() && !dialog.game.name_taken(name, Some(key));
                    if let Some(done) = rename_row(ui, name, focus, ok) {
                        action = done.then(|| CategoryDialogAction::Rename {
                            key: key.clone(),
                            name: name.trim().to_owned(),
                        });
                    }
                    continue;
                }
                let in_use = category.key == dialog.game.current;
                let selected = category.key == dialog.selected;
                match row(ui, &category.name, in_use, selected) {
                    Some(RowAction::Select) => select = Some(category.key.clone()),
                    Some(RowAction::Rename) => rename = Some(category.key.clone()),
                    None => {}
                }
            }
        });
    if let Some(key) = select {
        dialog.selected = key;
    }
    if let Some(key) = rename {
        dialog.start_rename(&key);
    }
    action
}

enum RowAction {
    Select,
    Rename,
}

/// A category in the list: its name, IN USE for the category in use, and
/// Rename. The selected row is outlined.
fn row(ui: &mut Ui, name: &str, in_use: bool, selected: bool) -> Option<RowAction> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::click());
    if selected || response.hovered() {
        let stroke = if selected {
            Stroke::new(1.0, theme::ACCENT)
        } else {
            Stroke::NONE
        };
        ui.painter()
            .rect(rect, theme::RADIUS, theme::RAISED, stroke, StrokeKind::Inside);
    }
    let inner = rect.shrink2(vec2(12.0, 0.0));
    let rename = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(inner)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let rename = ui.add(Button::secondary("Rename").size(ButtonSize::Xs));
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add(
                        Label::new(
                            RichText::new(name).font(theme::body(14.0)).color(theme::TEXT),
                        )
                        .selectable(false)
                        .truncate(),
                    );
                    if in_use {
                        ui.add(
                            Label::new(
                                RichText::new("IN USE")
                                    .font(theme::mono(11.0))
                                    .color(theme::TEXT_SECONDARY),
                            )
                            .selectable(false),
                        );
                    }
                });
                rename.clicked()
            },
        )
        .inner;
    if rename {
        Some(RowAction::Rename)
    } else if response.clicked() {
        Some(RowAction::Select)
    } else {
        None
    }
}

/// The row of the category being renamed: its name in a field, with Done.
/// Returns `Some(true)` when the name is kept, by Done or Enter, while it is
/// `ok`.
fn rename_row(ui: &mut Ui, name: &mut String, focus: &mut bool, ok: bool) -> Option<bool> {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let field = name_field(ui, name, focus, WIDTH - 72.0, "Category name");
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let done = ui.add(Button::secondary("Done").size(ButtonSize::Sm).enabled(ok));
        ((done.clicked() || entered) && ok).then_some(true)
    })
    .inner
}

/// + New category, or the new category's name field with Add and a note
/// saying what it starts with.
fn adding(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let in_use = dialog.name_of(&dialog.game.current).to_owned();
    let ok = match &dialog.mode {
        Mode::Adding { name, .. } => dialog.name_ok(name, None),
        _ => false,
    };
    let Mode::Adding { name, focus } = &mut dialog.mode else {
        let new = ui.add(
            Button::secondary("+ New category")
                .size(ButtonSize::Sm)
                .enabled(dialog.mode == Mode::List),
        );
        if new.clicked() {
            dialog.start_adding();
        }
        return None;
    };
    let action = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let field = name_field(ui, name, focus, WIDTH - 64.0, "Category name");
            let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
            let add = ui.add(Button::secondary("Add").enabled(ok));
            ((add.clicked() || entered) && ok)
                .then(|| CategoryDialogAction::Add(name.trim().to_owned()))
        })
        .inner;
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new(format!("Starts as a copy of the settings in use now ({in_use})."))
                .font(theme::body(12.0))
                .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    action
}

/// "Name this category": the game, the question, a name field and Not now
/// and Save (spec §6.8).
fn naming(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let ok = match &dialog.mode {
        Mode::Naming { key, name, .. } => dialog.name_ok(name, Some(key)),
        _ => false,
    };
    let game = dialog.game.name.clone();
    let Mode::Naming { key, name, focus } = &mut dialog.mode else {
        return None;
    };
    ui.label(
        RichText::new("NAME THIS CATEGORY")
            .font(theme::display(16.0))
            .color(theme::TEXT),
    );
    ui.add_space(8.0);
    ui.add(
        Label::new(
            RichText::new(game)
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
        )
        .truncate(),
    );
    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new(
                "These settings were saved before the bridge kept settings per category. \
                 Which category are they set up for?",
            )
            .font(theme::body(14.0))
            .color(theme::TEXT),
        )
        .wrap(),
    );
    ui.add_space(16.0);
    ui.add(
        Label::new(
            RichText::new("Category")
                .font(theme::body_semibold(13.0))
                .color(theme::TEXT),
        )
        .selectable(false),
    );
    ui.add_space(6.0);
    let field = name_field(ui, name, focus, WIDTH, "Any%");
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    ui.add_space(6.0);
    ui.add(
        Label::new(
            RichText::new("You can add more categories later. Not now keeps the name \"Default\".")
                .font(theme::body(12.0))
                .color(theme::TEXT_MUTED),
        )
        .wrap(),
    );
    ui.add_space(20.0);
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let save = ui.add(Button::primary("Save").enabled(ok));
        let not_now = ui.add(Button::secondary("Not now"));
        if (save.clicked() || entered) && ok {
            Some(CategoryDialogAction::Rename {
                key: key.clone(),
                name: name.trim().to_owned(),
            })
        } else if not_now.clicked() {
            Some(CategoryDialogAction::Cancel)
        } else {
            None
        }
    })
    .inner
}

/// A one-line name field `width` wide showing `hint` while empty, focused
/// the first time it shows.
fn name_field(
    ui: &mut Ui,
    name: &mut String,
    focus: &mut bool,
    width: f32,
    hint: &str,
) -> egui::Response {
    let field = ui.add(
        TextEdit::singleline(name)
            .font(theme::body(14.0))
            .hint_text(hint)
            .desired_width(width)
            .min_size(vec2(0.0, 32.0))
            .vertical_align(Align::Center)
            .margin(Margin::symmetric(10, 0)),
    );
    if *focus {
        field.request_focus();
        *focus = false;
    }
    field
}

/// "Delete <category> and its settings?", with Keep and Delete.
fn confirm_delete(ui: &mut Ui, dialog: &mut CategoryDialog) -> Option<CategoryDialogAction> {
    let name = dialog.name_of(&dialog.selected).to_owned();
    let frame = Frame::NONE
        .fill(theme::WINDOW)
        .stroke(Stroke::new(1.0, theme::BORDER_STRONG))
        .corner_radius(theme::RADIUS)
        .inner_margin(Margin::same(12));
    frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                Label::new(
                    RichText::new(format!(
                        "Delete {name} and its settings? This can't be undone."
                    ))
                    .font(theme::body(13.0))
                    .color(theme::TEXT),
                )
                .wrap(),
            );
            ui.add_space(8.0);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let delete = ui.add(Button::primary("Delete").size(ButtonSize::Sm));
                let keep = ui.add(Button::secondary("Keep").size(ButtonSize::Sm));
                if keep.clicked() {
                    dialog.back_to_list();
                }
                delete
                    .clicked()
                    .then(|| CategoryDialogAction::Delete(dialog.selected.clone()))
            })
            .inner
        })
        .inner
}
```

`theme::WINDOW` is the window background (`#0B0B0C`) and `ButtonSize::Sm` the 28 px button. The crate is edition 2024 on stable, so the `if let … && …` chain in `list` compiles.

- [ ] **Step 4: Change the Game dialog's note**

In `game_dialog.rs`, above the buttons (`ui.add_space(24.0);` before `let choice = …`), add the note Superdesign has:

```rust
ui.add_space(16.0);
ui.add(
    Label::new(
        RichText::new(
            "Settings are saved per game and category, so auto splitters for the same \
             game share them.",
        )
        .font(theme::body(12.0))
        .color(theme::TEXT_MUTED),
    )
    .wrap(),
);
```

and change the following `ui.add_space(24.0);` to `ui.add_space(20.0);`.

- [ ] **Step 5: Run tests, lint, commit**

Run: `cargo test --locked && cargo clippy --all-targets --locked -- -D warnings && cargo fmt --check`
Expected: PASS. `category_dialog` may warn as unused until Task 5; if clippy fails on `dead_code`, do Task 5 before committing and commit both together.

```bash
git add src/ui/components
git commit -m "feat(settings): add the category dialog"
```

---

### Task 5: Switching, adding, renaming and deleting categories in the app

**Files:**
- Modify: `src/ui/mod.rs` (`Pending`, `BridgeApp` field, `status_column`, new methods, `ui()`, tests)
- Modify: `src/ui/settings_integration.rs` (one new test)

**Interfaces:**
- Consumes: everything from Tasks 1–4.
- Produces: `Pending::ChangeCategory(String)`; `BridgeApp::change_category(&mut self)`, `category_action(&mut self, action: CategoryDialogAction)`, `switch_category(&mut self, key: &str)`, `ask_category_name(&mut self)`.

- [ ] **Step 1: Write the failing app tests**

In `src/ui/mod.rs` tests, add (they use `app_with_unsaved_edit`, whose game is "GTA", slug `gta`, category `default`):

```rust
fn table(text: &str) -> toml::Table {
    text.parse().unwrap()
}

/// `app_with_unsaved_edit`, with a second category "100%" whose gym is on.
fn app_with_two_categories() -> (tempfile::TempDir, BridgeApp) {
    let (dir, mut app) = app_with_unsaved_edit();
    let config = app.config.clone().unwrap();
    Game::add_category(&config, "gta", "100%", &table("gym = true")).unwrap();
    app.game = Some(Game::load(&config, "gta").unwrap());
    (dir, app)
}

fn saved_in(app: &BridgeApp, key: &str) -> toml::Table {
    let game = Game::load(app.config.as_ref().unwrap(), "gta").unwrap();
    game.categories
        .into_iter()
        .find(|category| category.key == key)
        .unwrap()
        .settings
}

#[test]
fn using_another_category_asks_first_with_unsaved_settings() {
    let (_dir, mut app) = app_with_two_categories();
    app.category_action(CategoryDialogAction::Use("100".to_owned()));
    assert_eq!(app.pending, Some(Pending::ChangeCategory("100".to_owned())));
    assert_eq!(app.game.as_ref().unwrap().current, "default");
}

#[test]
fn using_the_category_in_use_does_nothing() {
    let (_dir, mut app) = app_with_two_categories();
    app.category_action(CategoryDialogAction::Use("default".to_owned()));
    assert_eq!(app.pending, None);
    assert!(app.settings.is_unsaved());
}

#[test]
fn switching_writes_what_was_stored_then_loads_the_other_category() {
    let (_dir, mut app) = app_with_two_categories();
    app.settings.discard();
    // The auto splitter stored a value, not written yet.
    app.settings_changed(table("stored = 1"), Instant::now());
    app.proceed(Pending::ChangeCategory("100".to_owned()));

    assert_eq!(saved_in(&app, "default").get("stored"), Some(&Value::Integer(1)));
    assert_eq!(saved_in(&app, "100").get("stored"), None);
    let game = app.game.as_ref().unwrap();
    assert_eq!(game.current, "100");
    assert_eq!(app.settings.live().get("gym"), Some(&Value::Boolean(true)));
    assert_eq!(
        Game::load(app.config.as_ref().unwrap(), "gta").unwrap().current,
        "100"
    );
}

#[test]
fn saving_in_the_unsaved_dialog_saves_into_the_category_left() {
    let (_dir, mut app) = app_with_two_categories();
    app.category_action(CategoryDialogAction::Use("100".to_owned()));
    app.answer_unsaved(UnsavedAction::Save);
    assert_eq!(saved_in(&app, "default").get("gym"), Some(&Value::Boolean(true)));
    assert_eq!(app.game.as_ref().unwrap().current, "100");
    assert!(!app.settings.is_unsaved());
}

#[test]
fn a_new_category_copies_the_running_settings_not_the_edits() {
    let (_dir, mut app) = app_with_unsaved_edit();
    app.settings.set_live(table("level = 2"), Instant::now());
    app.category_dialog = Some(CategoryDialog::new(app.game.as_ref().unwrap()));
    app.category_action(CategoryDialogAction::Add("Low%".to_owned()));
    let added = saved_in(&app, "low");
    assert_eq!(added.get("level"), Some(&Value::Integer(2)));
    // "gym" is an unsaved edit.
    assert_eq!(added.get("gym"), None);
    // Adding doesn't switch.
    assert_eq!(app.game.as_ref().unwrap().current, "default");
    assert!(app.category_dialog.is_some());
}

#[test]
fn renaming_and_deleting_are_saved_straight_away() {
    let (_dir, mut app) = app_with_two_categories();
    app.category_dialog = Some(CategoryDialog::new(app.game.as_ref().unwrap()));
    app.category_action(CategoryDialogAction::Rename {
        key: "100".to_owned(),
        name: "100% NMG".to_owned(),
    });
    let game = Game::load(app.config.as_ref().unwrap(), "gta").unwrap();
    assert!(game.categories.iter().any(|category| category.name == "100% NMG"));

    app.category_action(CategoryDialogAction::Delete("100".to_owned()));
    let game = Game::load(app.config.as_ref().unwrap(), "gta").unwrap();
    assert_eq!(game.categories.len(), 1);
    assert_eq!(app.game.as_ref().unwrap().categories.len(), 1);
}

/// `app_with_unsaved_edit`, whose game's file is from before categories.
fn app_with_an_old_file() -> (tempfile::TempDir, BridgeApp) {
    let (dir, mut app) = app_with_unsaved_edit();
    let config = app.config.clone().unwrap();
    std::fs::write(config.game_file("gta"), "name = \"GTA\"\n\n[settings]\ngym = true\n")
        .unwrap();
    app.game = Some(Game::load(&config, "gta").unwrap());
    (dir, app)
}

#[test]
fn settings_from_before_categories_are_named_when_asked() {
    let (_dir, mut app) = app_with_an_old_file();
    app.ask_category_name();
    assert!(app.category_dialog.as_ref().is_some_and(CategoryDialog::is_naming));
    app.category_action(CategoryDialogAction::Rename {
        key: "default".to_owned(),
        name: "Any%".to_owned(),
    });
    // Named: the dialog closes rather than showing the list.
    assert!(app.category_dialog.is_none());
    let game = app.game.as_ref().unwrap();
    assert_eq!(game.category().name, "Any%");
    assert_eq!(game.unnamed, None);
    assert_eq!(saved_in(&app, "default").get("gym"), Some(&Value::Boolean(true)));
}

#[test]
fn not_now_keeps_default_and_leaves_the_file_alone() {
    let (_dir, mut app) = app_with_an_old_file();
    let config = app.config.clone().unwrap();
    let before = std::fs::read_to_string(config.game_file("gta")).unwrap();
    app.ask_category_name();
    app.category_action(CategoryDialogAction::Cancel);
    assert!(app.category_dialog.is_none());
    assert_eq!(std::fs::read_to_string(config.game_file("gta")).unwrap(), before);
    // Loading the game again asks again.
    app.game = Some(Game::load(&config, "gta").unwrap());
    app.ask_category_name();
    assert!(app.category_dialog.as_ref().is_some_and(CategoryDialog::is_naming));
}

#[test]
fn a_game_in_the_new_form_is_not_asked_about() {
    let (_dir, mut app) = app_with_unsaved_edit();
    app.ask_category_name();
    assert!(app.category_dialog.is_none());
}

#[test]
fn the_category_in_use_is_not_deleted() {
    let (_dir, mut app) = app_with_two_categories();
    app.category_dialog = Some(CategoryDialog::new(app.game.as_ref().unwrap()));
    app.category_action(CategoryDialogAction::Delete("default".to_owned()));
    let game = Game::load(app.config.as_ref().unwrap(), "gta").unwrap();
    assert_eq!(game.categories.len(), 2);
}
```

`SettingsEditor::set_live` returns the changed keys; its result can be ignored in the test (`let _ = …`) if it is `#[must_use]`. Extend `reload_open_and_change_game_ask_first_with_unsaved_settings` with `app.ask(Pending::ChangeCategory("100".to_owned())); assert_eq!(app.pending, Some(Pending::ChangeCategory("100".to_owned())));`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib ui::tests`
Expected: compile errors (no `category_action`, no `Pending::ChangeCategory`, no `category_dialog` field).

- [ ] **Step 3: Implement**

`Pending` gains a variant and its trigger:

```rust
    /// Switch to the loaded auto splitter's game's category with this key:
    /// its settings replace the draft.
    ChangeCategory(String),
…
            Self::ChangeCategory(_) => UnsavedTrigger::ChangeCategory,
```

`BridgeApp` gains, after `dialog`:

```rust
    /// The open Category dialog.
    category_dialog: Option<CategoryDialog>,
```

initialised to `None` in `new`. Import `CategoryDialog, CategoryDialogAction` from `components` (line ~41).

`proceed` gains `Pending::ChangeCategory(key) => self.switch_category(&key),`.

`status_column`: `Some(SplitterAction::ChangeCategory) => self.change_category(),`.

New methods, after `remember_game`:

```rust
    /// Opens the Category dialog for the loaded auto splitter's game. It
    /// asks about unsaved settings only when another category is used.
    fn change_category(&mut self) {
        if let Some(game) = &self.game {
            self.category_dialog = Some(CategoryDialog::new(game));
        }
    }

    /// Opens "Name this category" if the loaded game's settings are from
    /// before categories (spec §7.4). Called when an auto splitter is loaded
    /// and when its game is changed.
    fn ask_category_name(&mut self) {
        if let Some(game) = &self.game
            && let Some(key) = &game.unnamed
        {
            self.category_dialog = Some(CategoryDialog::naming(game, key));
        }
    }

    /// Shows the Category dialog while it is open, and acts on its answer.
    fn show_category_dialog(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.category_dialog else {
            return;
        };
        if let Some(action) = components::category_dialog(ctx, dialog) {
            self.category_action(action);
        }
    }

    /// Acts on what was done in the Category dialog (spec §6.8, §7.4).
    /// Adding, renaming and deleting are saved straight away; using
    /// another category closes the dialog and switches to it, once the
    /// unsaved settings are dealt with.
    fn category_action(&mut self, action: CategoryDialogAction) {
        let (Some(config), Some(slug)) = (
            self.config.clone(),
            self.game.as_ref().map(|game| game.slug.clone()),
        ) else {
            self.category_dialog = None;
            return;
        };
        // The result, the category to select, and what to log on success
        // and on failure.
        let (result, select, done, failed) = match action {
            CategoryDialogAction::Cancel => {
                self.category_dialog = None;
                return;
            }
            CategoryDialogAction::Use(key) => {
                self.category_dialog = None;
                if self.game.as_ref().is_some_and(|game| game.current != key) {
                    self.ask(Pending::ChangeCategory(key));
                }
                return;
            }
            CategoryDialogAction::Add(name) => {
                match Game::add_category(&config, &slug, &name, self.settings.live()) {
                    Ok((game, key)) => (
                        Ok(game),
                        Some(key),
                        format!("Added the category {name}"),
                        String::new(),
                    ),
                    Err(error) => (
                        Err(error),
                        None,
                        String::new(),
                        format!("Couldn't add the category {name}"),
                    ),
                }
            }
            CategoryDialogAction::Rename { key, name } => {
                let old = self.category_name(&key);
                (
                    Game::rename_category(&config, &slug, &key, &name),
                    None,
                    format!("Renamed {old} to {name}"),
                    format!("Couldn't rename {old}"),
                )
            }
            CategoryDialogAction::Delete(key) => {
                let name = self.category_name(&key);
                (
                    Game::delete_category(&config, &slug, &key),
                    None,
                    format!("Deleted the category {name}"),
                    format!("Couldn't delete the category {name}"),
                )
            }
        };
        match result {
            Ok(mut game) => {
                // For example "Renamed 100% to 100% NMG (GTA)".
                self.log(Category::App, format!("{done} ({})", game.name));
                // The category in use is the one running.
                if let Some(running) = &self.game {
                    game.current.clone_from(&running.current);
                }
                // Naming settings from before categories is done once named.
                if self.category_dialog.as_ref().is_some_and(CategoryDialog::is_naming) {
                    self.category_dialog = None;
                }
                if let Some(dialog) = &mut self.category_dialog {
                    dialog.refresh(&game, select.as_deref());
                }
                self.game = Some(game);
            }
            Err(error) => {
                self.log(Category::Error, format!("{failed}: {error}"));
                if let Some(dialog) = &mut self.category_dialog {
                    dialog.back_to_list();
                }
            }
        }
    }

    /// The name of the loaded game's category `key`, or the key.
    fn category_name(&self, key: &str) -> String {
        self.game
            .as_ref()
            .and_then(|game| game.categories.iter().find(|category| category.key == key))
            .map_or_else(|| key.to_owned(), |category| category.name.clone())
    }

    /// Switches the running auto splitter to its game's category `key`
    /// (spec §7.4): what it stored goes to the category it was using, then
    /// the new category's settings replace its map and the draft.
    fn switch_category(&mut self, key: &str) {
        let (Some(config), Some(slug)) = (
            self.config(),
            self.game.as_ref().map(|game| game.slug.clone()),
        ) else {
            return;
        };
        // What it stored so far belongs to the category it was using.
        self.write_settings();
        let game = match Game::use_category(&config, &slug, key) {
            Ok(game) => game,
            Err(error) => {
                self.log(
                    Category::Error,
                    format!("Couldn't switch the category: {error}"),
                );
                return;
            }
        };
        self.runner.set_settings(config::to_runtime(game.settings()));
        self.log(
            Category::App,
            format!("Using the settings of {} for {}", game.category().name, game.name),
        );
        self.settings
            .reset_keeping_widgets(as_runtime_has_it(game.settings()));
        self.stored_keys.clear();
        self.game = Some(game);
    }
```

Ask for the name of settings from before categories wherever a game is loaded:

- In `runner_event`, `RunnerEvent::Loaded` (~363), after `self.stored_keys.clear();`, add `self.ask_category_name();`. Starting the app loads the last auto splitter, so this covers it.
- In `game_dialog`, the `GameDialogKind::Change` arm (~600), after `self.game = Some(game);`, add `self.ask_category_name();`. The `GameDialogKind::New` arm needs nothing: it loads, and `Loaded` asks.

The Game dialog has closed (`self.dialog.take()`) by then, so the naming dialog comes after it, as spec §6.8 says.

In `fn ui` (around `self.game_dialog(&ui.ctx().clone());`, ~1341), add after it:

```rust
        self.show_category_dialog(&ui.ctx().clone());
```

- [ ] **Step 4: Run the app tests**

Run: `cargo test --locked --lib ui::tests`
Expected: PASS.

- [ ] **Step 5: Write the integration test**

In `src/ui/settings_integration.rs`, import `super::Pending` and add:

```rust
#[test]
fn switching_category_gives_the_auto_splitter_its_settings_and_remembers_it() {
    let mut harness = Harness::new();
    let path = harness.path.clone();
    harness.app.load(path);
    harness.until("the settings", ready);

    // The Category dialog adds a category from the running settings.
    harness.app.change_category();
    let nodes = harness.frame(Vec::new());
    for label in ["IN USE", "Default", "+ New category", "Use this category"] {
        assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
    }
    harness.click("+ New category");
    // The name field takes the focus on the frame after the click.
    harness.frame(Vec::new());
    harness.frame(vec![Event::Text("100%".to_owned())]);
    harness.click("Add");
    let game = Game::load(&harness.config, "gta").unwrap();
    let added = game.categories.iter().find(|category| category.key == "100").unwrap();
    assert_eq!(added.settings["best_route_version"].as_integer(), Some(7));

    // It is selected; using it switches without asking, as nothing is unsaved.
    Game::save_settings(&harness.config, "gta", "100", &"gym = true".parse().unwrap())
        .unwrap();
    harness.app.game = Some(Game::load(&harness.config, "gta").unwrap());
    harness.app.game.as_mut().unwrap().current = "default".to_owned();
    harness.app.proceed(Pending::ChangeCategory("100".to_owned()));
    let nodes = harness.until("the category's settings", |_, nodes| {
        nodes
            .iter()
            .any(|node| node.label == "Gym Moves" && node.toggled == Some(true))
    });
    assert!(has(&nodes, "100%"), "{nodes:#?}");

    // The category in use is remembered: a reload starts from it.
    harness.app.reload();
    let nodes = harness.until("the reloaded settings", ready);
    assert_eq!(find(&nodes, "Gym Moves").toggled, Some(true));
    assert_eq!(harness.app.game.as_ref().unwrap().current, "100");
}
```

And a test that loading a game from before categories asks, and Save names it:

```rust
#[test]
fn loading_settings_from_before_categories_asks_which_category_they_are_for() {
    let mut harness = Harness::new();
    std::fs::write(
        harness.config.game_file("gta"),
        "name = \"GTA\"\n\n[settings]\ngym = true\n",
    )
    .unwrap();
    let path = harness.path.clone();
    harness.app.load(path);
    harness.until("the naming dialog", |_, nodes| has(nodes, "NAME THIS CATEGORY"));
    // The name field takes the focus on the frame after it shows.
    harness.frame(Vec::new());
    harness.frame(vec![Event::Text("Any%".to_owned())]);
    // Enter, not a click: the Settings tab behind has its own "Save".
    harness.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    let game = Game::load(&harness.config, "gta").unwrap();
    assert_eq!(game.category().name, "Any%");
    assert_eq!(game.unnamed, None);
    assert_eq!(game.settings()["gym"].as_bool(), Some(true));
}
```

`Game::save_settings(…, "100", …)` records `100` as the category in use in the file, which is why the test sets the in-memory `current` back to `default` before switching: it stands for the app still running Default.

- [ ] **Step 6: Run every test, lint, commit**

Run: `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo build --locked && cargo test --locked`
Expected: PASS.

```bash
git add src/ui
git commit -m "feat(settings): switch, add, rename and delete categories"
```

---

### Task 6: User documentation

**Files:**
- Modify: `docs/wiki/Auto-Splitters-and-Settings.md`
- Modify: `docs/wiki/The-Main-Window.md`

No full release exists yet (only `v0.1.0-rc.*`), so the pages describe `main` directly and `Upcoming-Changes.md` isn't used (spec §15.3).

- [ ] **Step 1: Auto Splitters and Settings**

- Line 9: "The card then shows the file name, the game it's for and the category whose settings are in use."
- After the "Which game it's for" section, add:

```markdown
## Categories

Each game has one or more categories, such as Any% and 100%, and each category keeps its own settings. That way the same auto splitter can split on different things for different runs. A new game starts with one category, "Default". If you used the app before categories existed, the first time it loads one of your games it asks which category those settings are for, such as Any%, and names the category after your answer. Choose **Not now** to keep the name "Default"; it asks again next time, until the settings are saved, and you can rename the category later.

The Auto splitter card shows the category in use under the game. To change it, click the **Change** next to the category. The Category dialog lists the game's categories, and the one in use is marked IN USE.

- **Use another category:** select it and click **Use this category**. The auto splitter keeps running with that category's settings. If you have unsaved changes, the app asks about them first.
- **Add one:** click **+ New category**, type a name and click **Add**. It starts as a copy of the settings in use now, without unsaved changes.
- **Rename one:** click **Rename** on its row, type the new name, then press Enter or click **Done**. Escape keeps the old name.
- **Delete one:** select it, click **Delete**, then **Delete** again to confirm. You can't delete the category in use or a game's last category.

Adding, renaming and deleting are saved straight away. **Cancel** only closes the dialog. The app remembers the last category you used for each game and starts with it next time.

<!-- screenshot: S10 category-dialog.png -->
The Category dialog with three categories, Any% marked IN USE, and the + New category, Delete, Cancel and Use this category buttons.

LiveSplit One doesn't tell the app which category your splits are for, so choose it here.
```

- "Unsaved changes" section: "If you close the app, open another file, click **Reload**, click **Change** for the game, or use another category while there are unsaved changes, … for example "You changed 3 settings for GTA San Andreas (Any%). Reloading the auto splitter without saving discards them.""
- "Settings shared by a game" heading → "Settings shared by a game and category"; first sentence: "All auto splitters for the same game share one set of saved values for each category."

- [ ] **Step 2: The Main Window**

- Auto splitter card: "Shows the file name, its game with **Change**, the category in use with its own **Change**, and **Open…** and **Reload**."
- Compact paragraph: "A thin strip replaces the Auto splitter card, with the file name, the category and **Reload** … The file name is shortened when space is short, never the category."

- [ ] **Step 3: Check and commit**

Run: `scripts/wiki/check.sh`
Expected: no broken links or images (the S10 marker is a comment, not an image).

```bash
git add docs/wiki
git commit -m "docs: document categories in the wiki"
```

---

## Final checks and handover

- [ ] Run the whole CI set: `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo build --locked && cargo test --locked`.
- [ ] Launch the app (`cargo run`), load a real auto splitter, add a category, change a setting, switch back and forth, and restart. The category in use and each category's settings must persist.
- [ ] Push and open the pull request with `.github/pull_request_template.md`, `Closes #81`. Screenshots to retake: S1 `main-window.png` (card has a category line), S6 `compact-window.png` (strip shows the category), S3 `game-dialog.png` (new note line); new S10 `category-dialog.png`.
