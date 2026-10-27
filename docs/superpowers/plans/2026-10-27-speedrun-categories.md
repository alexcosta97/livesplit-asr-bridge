# speedrun.com Categories Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** For a game linked to speedrun.com, every category name field in
the Category dialog offers the game's speedrun.com variants (full-game, and
level ones behind a checkbox) and links the category to the variant picked,
stored in the game's file (issue #83).

**Architecture:** `src/speedrun/categories.rs` reads the API's game answer
into a flat list of `Variant`s, and a `CategorySource` trait fetches it on a
one-shot worker thread (a fake and an offline one for tests). A new
`src/ui/components/category_suggest.rs` holds the name field's suggestion
state and its drop-down, used by the three name fields of the Category
dialog. `RunCategory` gains an optional `CategoryLink`; `BridgeApp` passes
links through `add_category` and `rename_category`, and removes the
categories' links when the game's link changes.

**Tech Stack:** Rust 2024, egui/eframe 0.36, `ureq` 3, `serde_json`, `toml`,
`strsim`.

**Spec:** `docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md`:
§6.8 (Category bullet), §6.11, §7.1, §7.4, §7.5, **§7.5.1**, §8.1, §9, §10,
§12, §15.2.

## Global Constraints

- Endpoint: `GET https://www.speedrun.com/api/v1/games/<speedrun_id>?embed=levels.categories.variables,categories.variables`,
  asked once per Category dialog, only for a linked game.
- `User-Agent: livesplit-asr-bridge/<version>` (`crate::version::VERSION`),
  5-second timeout, as the game search.
- Full-game: categories with `type` `per-game` (miscellaneous included);
  subcategories are variables with `is-subcategory` and scope `global` or
  `full-game`.
- Levels: each level's own `categories`; subcategories are those with scope
  `global`, `all-levels`, or `single-level` whose `level` is that level.
- Every combination of one value per subcategory is a variant; order is
  speedrun.com's (levels, categories, variables, values as in the JSON).
- Names: `Any% (Unrestricted)`, `100% (PC, Glitchless)`,
  `Bob-omb Battlefield: Stage RTA (N64)`; no parentheses without values.
- File keys in `[categories.<key>]`: `speedrun_id`, `speedrun_values`
  (inline table variable id → value id, omitted when empty),
  `speedrun_level` (omitted for full-game); all omitted when not linked.
- Few/many threshold: 20 variants shown (level ones count while ticked).
- Copy: `Type to search <count> categories`,
  `Use "<typed name>" (not on speedrun.com)`, `Linked to speedrun.com`,
  `Include level categories`.
- List: 5 rows of 32 px, scrolls; SET UP tag; dimmed rows can't be picked.
- Narrowing: variant names containing the typed text, letters and digits
  only, ignoring case.
- Failures are logged in `Category::App` once per dialog, never shown.
- No test makes a real network request.
- Conventional Commits, signed. Run `cargo fmt`,
  `cargo clippy --all-targets --locked -- -D warnings` and
  `cargo test --locked` before each commit.

## Review Focus

1. A variable's `values.values` is a JSON object; serde_json without
   `preserve_order` sorts it. Values must keep the JSON's order without
   turning on `preserve_order` (the server protocol uses serde_json too).
   Test in Task 1.
2. Missing pieces in the answer: a game with no `levels`, a level with no
   `categories`, a category with no `variables`, a subcategory with no
   values. No panic; a subcategory without values is skipped rather than
   wiping out the category. A body without `data` is an error. Test in
   Task 1.
3. An answer arriving after the dialog closed, or for a game whose dialog
   was replaced (Change game while the Category dialog was open), is
   ignored, and closing never blocks on the worker. Test in Tasks 2 and 5.
4. Enter that picks a highlighted row must not also press **Add**, **Done**
   or **Save** in the same frame, and Escape with the list open must not
   leave the edit or close the dialog. Test in Task 4.
5. A hand-edited file with `speedrun_values` but no `speedrun_id` reads as
   not linked rather than failing. Test in Task 3.

---

### Task 0: Superdesign CategoryDialog states (coordinator, not a subagent)

The coordinating session does this with the `superdesign` skill before
Task 4. Superdesign is the design source of truth.

- [ ] Update the `CategoryDialog` component
  (`cece81d6-072d-4e91-a46d-90bf61420ce2`) with the §6.8 states: adding
  with a few variants listed (one SET UP), adding with more than 20 ("Type
  to search 214 categories"), renaming linked (linked line, **Change**),
  renaming with dimmed rows, the **Include level categories** checkbox
  under the field, the typed-name-only list, and "Name this category" with
  the list.
- [ ] Update `.superdesign/resume.json` if a draft version changes, and
  commit: `docs(design): add speedrun.com states to the category dialog`.
- [ ] Give the Task 4 implementer the component URL
  (`https://p.superdesign.dev/draftcomponent/cece81d6-072d-4e91-a46d-90bf61420ce2`).

### Task 1: Reading variants from the API answer

**Files:**
- Create: `src/speedrun/categories.rs`
- Modify: `src/speedrun/mod.rs` (module, re-exports)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Default)]
  pub struct CategoryLink {
      pub id: String,                          // speedrun.com category id
      pub values: BTreeMap<String, String>,    // variable id -> value id
      pub level: Option<String>,               // level id for a level variant
  }
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Variant {
      pub name: String,
      pub link: CategoryLink,
  }
  impl Variant { pub fn is_level(&self) -> bool } // link.level.is_some()
  pub fn parse_variants(json: &str) -> Result<Vec<Variant>, String>;
  ```
  Full-game variants first, then level variants, all in speedrun.com's
  order.

- [ ] **Step 1: Write the failing tests** in `categories.rs`, with a fixture
  string built from the real shape (see the spec's Global Constraints):
  a game with categories `Any%` (per-game, subcategory `Platform` global
  with values `PC`, `N64` in that JSON order, plus a non-subcategory
  `Verified`), `0 Star` (per-game, no variables), `Stage RTA` (per-level,
  at top level too), and two levels, `Bob-omb Battlefield` and
  `Whomp's Fortress`, each with `categories` holding `Stage RTA` whose
  variables are `Platform` (global), `Codes` (all-levels, subcategory, one
  value `On`) and `Star` (single-level, subcategory, `level` =
  Whomp's Fortress id, values `1`, `2`).
  - `reads_full_game_variants_in_order`: the first variants are
    `Any% (PC)`, `Any% (N64)`, `0 Star`, with `values` `{platform: pc}`
    and `level == None`; `Stage RTA` appears only as level variants.
  - `level_variants_take_only_their_level_s_subcategories`:
    `Bob-omb Battlefield: Stage RTA (PC, On)` exists, and no Bob-omb
    variant has a `Star` value; `Whomp's Fortress: Stage RTA (PC, On, 1)`
    exists with `level == Some(<whomp id>)`.
  - `values_keep_the_order_of_the_answer`: with values given in the JSON as
    `zz` then `aa`, the variants come out `zz` first (Review Focus 1).
  - `missing_parts_read_as_empty` (Review Focus 2): no `levels`, a level
    without `categories`, a category without `variables`, a subcategory
    with no values (skipped: the category is still one variant) all parse.
  - `an_answer_without_data_is_an_error`: `{"status":420}` and `not json`.

- [ ] **Step 2: Run** `cargo test --locked speedrun::categories` and see the
  tests fail to compile.

- [ ] **Step 3: Implement** `parse_variants` with private serde structs for
  the answer. Read `values.values` into an ordered `Vec<(String, String)>`
  of (value id, label) with a small `Deserialize` impl that walks the map
  with `MapAccess` (do not enable serde_json's `preserve_order`). Variant
  names: category name, then ` (` + labels joined by `, ` + `)` when there
  are any; level variants prefix `<level name>: `.

- [ ] **Step 4: Run** `cargo test --locked speedrun` and see it pass.

- [ ] **Step 5: Commit** `feat(speedrun): read a game's category variants`.

### Task 2: Fetching variants, and matching against them

**Files:**
- Modify: `src/speedrun/categories.rs` (the source), `src/speedrun/matching.rs`,
  `src/speedrun/search.rs` (share the agent setup), `src/speedrun/mod.rs`

**Interfaces:**
- Consumes: `parse_variants`, `Variant` (Task 1); `API_BASE`, `TIMEOUT`,
  `VERSION` from `search.rs`.
- Produces:
  ```rust
  pub trait CategorySource: Send {
      /// Starts fetching the variants of the speedrun.com game `game_id`.
      fn request(&mut self, game_id: &str);
      /// The answer, once: the game id asked for and its variants or error.
      fn poll(&mut self) -> Option<(String, Result<Vec<Variant>, String>)>;
  }
  #[cfg(not(test))] pub fn live_categories() -> Box<dyn CategorySource>;
  #[cfg(test)] pub struct OfflineCategories;          // never answers
  #[cfg(test)] #[derive(Clone, Default)] pub struct FakeCategories; // shared state
  impl FakeCategories {
      pub fn answer(&self, game_id: &str, variants: Result<Vec<Variant>, String>);
      pub fn requests(&self) -> Vec<String>;
  }
  pub fn clear_variant(name: &str, variants: &[&Variant]) -> Option<usize>;
  ```

- [ ] **Step 1: Write the failing tests.** In `categories.rs`, using the
  local HTTP server helper from `search.rs`'s tests (move `serve` and `ok`
  into a `#[cfg(test)] pub(super) mod test_server` in `search.rs` or a new
  `src/speedrun/test_server.rs`, whichever keeps both test modules short):
  - `asks_for_the_game_with_levels_and_categories_embedded`: the request
    line is `GET /games/abc?embed=levels.categories.variables,categories.variables`
    (decoded), with the app's user agent; the answer is `("abc", Ok(_))`.
  - `an_error_status_is_a_failed_answer` (HTTP 420).
  - `dropping_the_source_does_not_wait_for_the_worker` (< 100 ms with an
    unanswered request).
  In `matching.rs`:
  - `a_variant_matches_clearly_by_its_name_only`: `"any%(pc)"` against
    `Any% (PC)`, `Any% (N64)` is `Some(0)`; `"Any%"` against both is
    `None` (both below 0.9 or both equal); `"!!!"` is `None`.

- [ ] **Step 2: Run** `cargo test --locked speedrun` and see them fail.

- [ ] **Step 3: Implement.** `CategorySource` for a `SpeedrunCategories`
  that spawns one thread per `request`, sending the answer down an
  `mpsc` channel; a send to a dropped receiver is ignored. Build the
  `ureq::Agent` with the same helper the game search uses (extract
  `fn agent() -> ureq::Agent` in `search.rs`). `clear_variant` reuses
  `normalize`, `similarity` and `CLEAR_MATCH`, against the variant's name
  only.

- [ ] **Step 4: Run** `cargo test --locked speedrun` and see it pass.

- [ ] **Step 5: Commit** `feat(speedrun): fetch a game's category variants`.

### Task 3: Category links in the game's file

**Files:**
- Modify: `src/config/games.rs`, every `RunCategory { .. }` literal in
  `src/` (add `speedrun: None`)

**Interfaces:**
- Consumes: `CategoryLink` (Task 1).
- Produces:
  ```rust
  pub struct RunCategory { pub key, pub name, pub settings, pub speedrun: Option<CategoryLink> }
  impl Game {
      pub fn add_category(config, slug, name, settings: &Table, link: Option<&CategoryLink>)
          -> Result<(Self, String), String>;
      pub fn rename_category(config, slug, key, name, link: Option<&CategoryLink>)
          -> Result<Self, String>; // sets the link; None unlinks
      pub fn linked_to(&self, link: &CategoryLink) -> Option<&RunCategory>;
  }
  ```
  `set_link` removes every category's link when the game's `speedrun_id`
  changes (another id or `None`); the same id keeps them.

- [ ] **Step 1: Write the failing tests** in `games.rs`:
  - `a_category_link_is_written_and_read_back`: add `Any% (PC)` with
    `{id: "jdzg83rk", values: {"yn2wk42n": "21gzep8l"}, level: None}`;
    the file text contains `speedrun_id = "jdzg83rk"` and
    `speedrun_values = { yn2wk42n = "21gzep8l" }` (or the equivalent
    sub-table form `toml` writes; assert by parsing the file as a
    `toml::Table`), and no `speedrun_level`; loading gives the same link.
  - `an_unlinked_category_writes_no_speedrun_keys`.
  - `a_level_link_keeps_its_level`.
  - `renaming_without_a_link_unlinks_and_keeps_the_name`.
  - `a_link_another_category_has_is_refused` by add and rename (`Err`).
  - `changing_the_game_s_link_unlinks_its_categories`: relink to another
    id → no category linked; relink to the same id → links kept; unlink →
    none.
  - `values_without_an_id_read_as_not_linked` (Review Focus 5).

- [ ] **Step 2: Run** `cargo test --locked config::games` and see them fail.

- [ ] **Step 3: Implement.** `CategoryFile` gains
  `speedrun_id: Option<String>`, `speedrun_values: BTreeMap<String,String>`
  (`default`, skip if empty) and `speedrun_level: Option<String>`, all
  `skip_serializing_if` empty, declared before `settings` so they're
  written above the settings table. Update the callers in `src/ui/mod.rs`
  to pass `None` for now so the build stays green.

- [ ] **Step 4: Run** `cargo test --locked` and see it pass.

- [ ] **Step 5: Commit** `feat(config): store a category's speedrun.com link`.

### Task 4: The suggestions in the Category dialog

**Files:**
- Create: `src/ui/components/category_suggest.rs` (state, narrowing, keys,
  drop-down drawing)
- Modify: `src/ui/components/category_dialog.rs`, `src/ui/components/mod.rs`,
  `src/ui/mod.rs` (only what keeps it compiling: the new action shapes,
  passing `link.as_ref()` to `add_category` / `rename_category`, and a
  `new_categories: fn() -> Box<dyn CategorySource>` field, set like
  `new_search` to `OfflineCategories` in tests and `live_categories`
  otherwise, passed to `CategoryDialog::new` and `::naming`)

**Interfaces:**
- Consumes: `CategorySource`, `Variant`, `CategoryLink`, `clear_variant`
  (Tasks 1–2); `RunCategory.speedrun`, `Game::linked_to` (Task 3);
  `Checkbox` component; the Game dialog's list look (`results_list`,
  `typed_row`, `linked_line` in `game_dialog.rs`) to match.
- Produces:
  ```rust
  pub enum CategoryDialogAction {
      Use(String),
      Add { name: String, link: Option<CategoryLink> },
      Rename { key: String, name: String, link: Option<CategoryLink> },
      Delete(String),
      Cancel,
  }
  impl CategoryDialog {
      /// `source` is asked for the game's variants when the game is linked.
      pub fn new(game: &Game, source: Box<dyn CategorySource>) -> Self;
      pub fn naming(game: &Game, key: &str, source: Box<dyn CategorySource>) -> Self;
      pub fn refresh(&mut self, game: &Game, select: Option<&str>); // keeps the variants
      pub fn take_failure(&mut self) -> Option<String>; // once per dialog
  }
  pub fn category_dialog(ctx: &egui::Context, dialog: &mut CategoryDialog)
      -> Option<CategoryDialogAction>;
  ```
  `category_suggest.rs` exposes a `Suggest` struct (one per open name
  field: `link`, `list_open`, `highlight`, `declined`, `may_link_on_own`)
  with pure methods the tests drive: `typed`, `pick(index)`, `pick_typed`,
  `change_link`, `key(Key) -> bool`, `rows(&variants, levels) -> Rows`
  where `Rows` is either `TypeToSearch(count)` or the narrowed variants.

- [ ] **Step 1: Write the failing tests** (headless, state-level, in
  `category_dialog.rs` and `category_suggest.rs`; a `Game` with
  `speedrun_id: Some("g")` and a `FakeCategories`):
  - `asks_for_the_variants_only_for_a_linked_game`: `requests() ==
    ["g"]`; an unlinked game asks nothing and its field shows no list.
  - `a_few_variants_are_listed_with_the_field_empty`: 3 variants →
    all 3 rows; 21 → `TypeToSearch(21)`; typing `"pc"` with 21 →
    only the variants containing `pc` (letters and digits, any case).
  - `level_variants_appear_only_while_ticked_and_count_towards_twenty`:
    19 full-game + 5 level variants: unticked → 19 rows; ticked →
    `TypeToSearch(24)`.
  - `picking_fills_the_name_and_links_it` then `typing_removes_the_link`,
    `change_removes_the_link_and_opens_the_list`,
    `the_last_row_keeps_the_typed_name_unlinked_and_is_not_linked_again`.
  - `add_returns_the_link`: Enter with no highlight →
    `Add { name: "Any% (PC)", link: Some(_) }`.
  - `picking_a_set_up_variant_while_adding_uses_that_category`: the game
    has `any` linked to `Any% (PC)`'s link, renamed `"Old"` → picking it
    returns `Rename { key: "any", name: "Any% (PC)", link: Some(_) }` and
    `selected == "any"`; same for a variant whose name an unlinked
    category has (ignoring case).
  - `rows_another_category_has_are_dimmed_while_renaming`: picking such a
    row index does nothing.
  - `rename_on_a_linked_category_starts_linked`, and
    `rename_on_an_unlinked_category_links_a_clear_match_on_its_own` once
    variants arrive (`"any% pc"` → `Any% (PC)`), but not after the last
    row declined it.
  - `the_level_box_starts_ticked_for_a_level_linked_category`.
  - `enter_on_a_highlighted_row_picks_it_without_adding` and
    `escape_closes_the_list_before_leaving_the_edit` (Review Focus 4),
    using `Suggest::key` returning `true` for consumed keys.
  - `a_failure_is_reported_once_and_the_list_has_only_the_last_row`.
  - Keep every existing test passing (update constructors to pass
    `Box::new(OfflineCategories)`).

- [ ] **Step 2: Run** `cargo test --locked ui::components` and see them fail.

- [ ] **Step 3: Implement.** The dialog polls its source every frame
  (`ctx.request_repaint_after(100 ms)` while waiting), keeps the variants
  for its lifetime (also across `refresh`), and creates a fresh `Suggest`
  when a name field opens. Feed list keys before the field like
  `game_dialog::list_keys`. The drop-down is an `Area` at `Order::Tooltip`
  under the field, styled as the Game dialog's (`RAISED`, `BORDER_STRONG`,
  shadow), rows 32 px, 5 shown; SET UP in mono 11 `TEXT_SECONDARY` at the
  right; dimmed rows in `TEXT_MUTED` with `Sense::hover()`; the
  "Type to search" row muted and not clickable. The linked line and the
  checkbox go under the field (the checkbox only while any variant is a
  level variant). Field width: in the rename row the list spans the
  field's width.

- [ ] **Step 4: Run** `cargo test --locked` and see it pass.

- [ ] **Step 5: Commit** `feat(ui): offer speedrun.com categories in the category dialog`.

### Task 5: The app: links, logging, and the game's link changing

**Files:**
- Modify: `src/ui/mod.rs`

**Interfaces:**
- Consumes: everything above, including `BridgeApp::new_categories`
  (Task 4).

- [ ] **Step 1: Write the failing tests** in `src/ui/mod.rs`'s tests, using
  `app_in_temp_dir`, `draw_app` and `lines`, and setting
  `app.category_dialog = Some(CategoryDialog::new(game, Box::new(fake)))`
  with a `FakeCategories`, as `a_failed_search_is_logged_once_in_app`
  does for the Game dialog:
  - `adding_a_linked_category_stores_its_link_and_logs_it`: log line
    `Linked Any% (PC) to speedrun.com (GTA)` in `Category::App`.
  - `renaming_without_a_link_unlinks_it`.
  - `using_a_set_up_variant_while_adding_renames_and_selects_it`.
  - `a_category_fetch_failure_is_logged_once`:
    `Couldn't get categories from speedrun.com: <error>`.
  - `changing_the_game_s_link_removes_its_categories_links` through the
    Game dialog's `apply_game_choice`.
  - `the_category_dialog_of_an_unlinked_game_asks_nothing`.

- [ ] **Step 2: Run** `cargo test --locked ui::tests` and see them fail.

- [ ] **Step 3: Implement.** `category_action` logs `Linked <name> to
  speedrun.com` or `Unlinked <name> from speedrun.com` alongside the
  existing add/rename lines when the link changed; `show_category_dialog`
  logs `take_failure()`. `link_game` relies on `Game::set_link` removing
  the categories' links and refreshes `self.game` from its result.

- [ ] **Step 4: Run** the full checks (`cargo fmt --check`, clippy, `cargo
  test --locked`) and see them pass.

- [ ] **Step 5: Commit** `feat(ui): link categories to speedrun.com`.

### Task 6: User documentation

**Files:**
- Modify: `docs/wiki/Auto-Splitters-and-Settings.md` (Categories section),
  `docs/wiki/Troubleshooting.md` if it covers speedrun.com offline,
  `docs/wiki/Logs-and-Files.md` (the game file example, if it has one)

No full release exists, so pages describe `main` directly (§15.3).

- [ ] **Step 1:** Add a "Picking a category from speedrun.com" subsection
  under Categories: only for a linked game; the list in **+ New
  category**, **Rename** and "Name this category"; a few listed vs "Type
  to search"; **Include level categories**; SET UP and dimmed rows; the
  last row; the linked line and **Change**; Rename linking on its own;
  links removed when the game's link changes; what is sent (the game's
  speedrun.com id, only while the dialog is open); offline.
- [ ] **Step 2:** Run `scripts/wiki/check.sh` and see it pass.
- [ ] **Step 3: Commit** `docs(wiki): document speedrun.com categories`.
