# speedrun.com Game Suggestions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Game dialog's name field searches speedrun.com, links the game
to its speedrun.com id (stored as `speedrun_id` in the game's file) and names
it as on speedrun.com (issue #82).

**Architecture:** A new `src/speedrun/` module holds the API types, the
response parsing, the match scoring and a background search worker behind a
`GameSearch` trait. The Game dialog owns a `Box<dyn GameSearch>`, debounces
typing with egui's input time, and keeps "which game" and "which speedrun.com
link" as two separate parts of its choice. `BridgeApp` applies the choice:
it sets up or selects the game, then writes, changes or removes its link.

**Tech Stack:** Rust 2024, egui/eframe 0.36, `ureq` 3 (blocking HTTPS on
the worker thread, rustls), `strsim` (edit distance), `serde_json`.

**Spec:** `docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md`:
§6.8 (Game dialog bullet), §6.11, §7.1, §7.3, **§7.5**, §8.1, §9, §10, §12,
§15.2, §15.4.

## Global Constraints

- Endpoint: `GET https://www.speedrun.com/api/v1/games?name=<name>&max=20`.
- `User-Agent: livesplit-asr-bridge/<version>`, with `crate::version::VERSION`.
- Timeout: 5 seconds per request.
- Debounce: 300 ms after typing stops. Blank names are never searched.
- Clear match: score ≥ 0.9, and no other result ≥ 0.9.
- The list shows 5 rows (32 px each, like the games list) and scrolls. Its
  fixed last row reads `Use "<typed name>" (not on speedrun.com)`.
- The linked line reads `Linked to speedrun.com`, with a secondary
  **Change** button (24 px).
- File key: `speedrun_id`, top level, after `name`, omitted when not linked.
- Failures are never shown as errors. They are logged in `Category::App`,
  once per dialog.
- No test makes a real network request.
- Every commit follows Conventional Commits and is signed. Run `cargo fmt`,
  `cargo clippy --all-targets --locked -- -D warnings` and
  `cargo test --locked` before each commit.

## Review Focus

1. Names with no letters or digits ("!!!", " ") or only non-ASCII ones
   ("ポータル"): no panic, and the name is URL-encoded. A name that
   normalizes to nothing scores 0 against every result. Test in Task 1.
2. An answer arriving after the field changed, or after the dialog closed,
   is ignored. Closing the dialog never blocks the UI on the worker thread.
   Test in Tasks 2 and 4.
3. API data with missing or `null` fields (`abbreviation`, `released`)
   parses, with `""` and `None`. A response without `data` is an error.
   Test in Task 1.
4. Enter that picks a highlighted row must not also press **Use this game**
   in the same frame. Test in Task 4.
5. Linking an existing game renames it but keeps its slug, so
   `splitters.toml` associations stay valid. Test in Task 3.

---

### Task 0: Superdesign GameDialog states (coordinator, not a subagent)

The coordinating session does this with the `superdesign` skill before
Task 4. Superdesign is the design source of truth.

- [ ] Update the `GameDialog` component
  (`4f5ae936-edb3-4768-bcb3-d33762a0036b`) with the states from spec §6.11:
  linked (the linked line with **Change**), searching with results (the
  dropdown over the content below, 5 rows, year in mono muted, `SET UP` tag,
  the fixed last row), searching with no results (the last row only), and
  not linked.
- [ ] Update `.superdesign/resume.json` if the draft version changes, then
  commit: `docs(design): add speedrun.com states to the game dialog`.
- [ ] Give the Task 4 implementer the component URL
  (`https://p.superdesign.dev/draftcomponent/4f5ae936-edb3-4768-bcb3-d33762a0036b`).

### Task 1: speedrun.com types, parsing and matching

**Files:**
- Create: `src/speedrun/mod.rs` (types, `parse_games`, re-exports)
- Create: `src/speedrun/matching.rs` (`score`, `clear_match`)
- Modify: `src/main.rs` (add `mod speedrun;`), `Cargo.toml` (add `strsim`)

**Interfaces:**
- Produces:
  - `pub struct SpeedrunGame { pub id: String, pub name: String, pub abbreviation: String, pub year: Option<u16> }`
    (`Debug, Clone, PartialEq, Eq`). `name` is `names.international`, and
    `year` is `released`.
  - `pub struct SpeedrunLink { pub id: String, pub name: String }`
    (`Debug, Clone, PartialEq, Eq`), with
    `impl From<&SpeedrunGame> for SpeedrunLink`.
  - `pub fn parse_games(json: &str) -> Result<Vec<SpeedrunGame>, String>`
  - `pub fn score(name: &str, game: &SpeedrunGame) -> f64`
  - `pub fn clear_match(name: &str, games: &[SpeedrunGame]) -> Option<usize>`
  - `pub const CLEAR_MATCH: f64 = 0.9;`

- [ ] **Step 1: Write the failing tests** in `src/speedrun/mod.rs` and
  `matching.rs`:

```rust
#[test]
fn reads_the_games_of_a_search() {
    let json = r#"{"data":[
      {"id":"4pd0n31e","names":{"international":"Portal","japanese":null},"abbreviation":"portal","released":2007},
      {"id":"x","names":{"international":"Odd"},"abbreviation":null,"released":null}],"pagination":{}}"#;
    assert_eq!(parse_games(json).unwrap(), vec![
        SpeedrunGame { id: "4pd0n31e".into(), name: "Portal".into(), abbreviation: "portal".into(), year: Some(2007) },
        SpeedrunGame { id: "x".into(), name: "Odd".into(), abbreviation: String::new(), year: None },
    ]);
}
#[test]
fn a_response_without_data_is_an_error() {
    assert!(parse_games(r#"{"status":420}"#).is_err());
    assert!(parse_games("not json").is_err());
}
// matching.rs
#[test]
fn same_letters_and_digits_match_fully() {
    assert_eq!(score("grand theft auto san andreas", &game("Grand Theft Auto: San Andreas", "gtasa")), 1.0);
    assert_eq!(score("GTA-SA", &game("Grand Theft Auto: San Andreas", "gtasa")), 1.0);
}
#[test]
fn otherwise_the_score_is_the_edit_similarity() {
    // 1 - 1/7
    assert!((score("portal", &game("Portals", "portals")) - 6.0 / 7.0).abs() < 1e-9);
}
#[test]
fn a_name_without_letters_or_digits_matches_nothing() {
    assert_eq!(score("!!!", &game("!!!", "")), 0.0);
    assert_eq!(score("ポータル", &game("ポータル", "")), 1.0);
}
#[test]
fn one_clear_match_is_picked() {
    let games = [game("Portal", "portal"), game("Portals", "portals"), game("Portal 2", "portal_2")];
    assert_eq!(clear_match("portal", &games), Some(0));
}
#[test]
fn two_clear_matches_or_none_pick_nothing() {
    let games = [game("Mario Kart", "mk"), game("Mario Kart DS", "mariokart")];
    assert_eq!(clear_match("mario kart", &games), None);
    assert_eq!(clear_match("gta sa de", &[game("Grand Theft Auto: San Andreas", "gtasa")]), None);
    assert_eq!(clear_match("portal", &[]), None);
}
```

(`game(name, abbreviation)` is a test helper with `id = abbreviation`,
`year = None`.)

- [ ] **Step 2:** Run `cargo test speedrun`. Expected: the tests fail to
  compile.
- [ ] **Step 3: Implement.** Parse with `serde_json` into private
  `#[derive(Deserialize)]` structs, with `Option` fields for `abbreviation`
  and `released`. Normalize with `char::is_alphanumeric` after
  `to_lowercase` (Unicode-aware, as in `config::names::slugify`). `score` is
  the maximum over the name and the abbreviation: `1.0` when the
  normalized strings are equal, otherwise `strsim::normalized_levenshtein`,
  and `0.0` when the normalized name is empty.
- [ ] **Step 4:** Run `cargo test speedrun`. Expected: PASS.
- [ ] **Step 5: Commit**
  `feat(speedrun): read speedrun.com game searches and score matches`.

### Task 2: Search worker

**Files:**
- Create: `src/speedrun/search.rs`
- Modify: `src/speedrun/mod.rs` (re-export), `Cargo.toml` (add `ureq` 3 with
  its default rustls features)

**Interfaces:**
- Consumes: `parse_games`, `SpeedrunGame` (Task 1); `crate::version::VERSION`.
- Produces:
  - `pub struct SearchAnswer { pub name: String, pub games: Result<Vec<SpeedrunGame>, String> }`
  - `pub trait GameSearch: Send { fn request(&mut self, name: &str); fn poll(&mut self) -> Option<SearchAnswer>; }`
  - `pub struct SpeedrunSearch`, with `pub fn start(base_url: &str) -> Self`
    (spawns the worker) and `impl GameSearch`.
  - `pub const API_BASE: &str = "https://www.speedrun.com/api/v1";`
  - `pub fn live() -> Box<dyn GameSearch>` (`SpeedrunSearch::start(API_BASE)`)
  - `pub struct Offline;` with `impl GameSearch` that never answers, used
    by app tests.
  - `#[cfg(test)] pub struct FakeSearch { pub requests: Arc<Mutex<Vec<String>>>, answers: VecDeque<SearchAnswer> }`
    with `pub fn answer(&mut self, name, games)` to queue an answer, and
    `poll` popping queued answers. Task 4 uses it.

- [ ] **Step 1: Write the failing tests.** Use a `std::net::TcpListener` on
  `127.0.0.1:0` serving one canned HTTP/1.1 response per connection (a
  small helper in the test module), and `SpeedrunSearch::start(&format!("http://{addr}"))`.

```rust
#[test]
fn asks_for_twenty_games_by_name_with_the_app_s_user_agent() {
    // server records the request line and headers, answers 200 with one game
    // request("gta san andreas"); poll until Some (deadline 5 s)
    // assert request line starts with "GET /games?name=gta%20san%20andreas&max=20"
    //   (or `+` for the space; assert on the decoded query instead if ureq encodes differently)
    // assert a header line equals format!("user-agent: livesplit-asr-bridge/{VERSION}") ignoring case
    // assert answer.name == "gta san andreas" and answer.games == Ok(vec![...])
}
#[test]
fn an_error_status_is_a_failed_answer() {
    // server answers "HTTP/1.1 420 \r\n..." -> answer.games.is_err()
}
#[test]
fn a_newer_name_replaces_one_still_waiting() {
    // server holds the first response until a channel is signalled;
    // request("a"), request("b"), request("c"), then release;
    // collect answers: names == ["a", "c"]
}
#[test]
fn dropping_the_search_does_not_wait_for_the_worker() {
    // server never answers; request("a"); let t = Instant::now(); drop(search);
    // assert t.elapsed() < Duration::from_millis(100)
}
```

- [ ] **Step 2:** Run `cargo test speedrun::search`. Expected: FAIL (no
  such items).
- [ ] **Step 3: Implement.** The worker thread owns a
  `ureq::Agent` built with `timeout_global(Some(Duration::from_secs(5)))`
  and the user agent. Shared state is an `Arc<(Mutex<State>, Condvar)>`,
  where `State { waiting: Option<String>, closed: bool }`. `request`
  replaces `waiting` and notifies. The worker takes `waiting`, runs it,
  sends a `SearchAnswer` on an `mpsc` channel, and exits when `closed` is
  set or the channel is gone. `Drop` sets `closed` and notifies, without
  joining. Build the URL with ureq's `.query("name", name).query("max", "20")`
  so the name is encoded. Turn transport errors and non-2xx statuses into
  `Err(String)`.
- [ ] **Step 4:** Run `cargo test speedrun`. Expected: PASS. Then run
  `cargo build --locked` after `cargo update -p ureq -p strsim` has written
  `Cargo.lock`.
- [ ] **Step 5: Commit** `feat(speedrun): search speedrun.com games in the background`.

### Task 3: `speedrun_id` in the game's file

**Files:**
- Modify: `src/config/games.rs` (`Game`, `GameSummary`, `GameFile`, `load`,
  `write`, `list`, new `set_link`, tests). Every `Game { .. }` and
  `GameSummary { .. }` literal in `src/` gains the new field: compile errors
  list them (including `ui/mod.rs` and `game_dialog.rs` tests).

**Interfaces:**
- Consumes: `crate::speedrun::SpeedrunLink` (Task 1).
- Produces:
  - `Game.speedrun_id: Option<String>`. `Game::empty` gives `None`.
  - `GameSummary.speedrun_id: Option<String>`
  - `pub fn set_link(config: &Config, slug: &str, link: Option<&SpeedrunLink>) -> Result<Game, String>`:
    reads the file again, and sets `speedrun_id`. With `Some`, it also sets
    `name = link.name`. With `None`, it keeps the name. Then it writes the
    file.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_linked_game_keeps_its_speedrun_id() {
    // write "name = \"Portal\"\nspeedrun_id = \"4pd0n31e\"\n" by hand; load -> speedrun_id == Some("4pd0n31e")
    // save_settings on it; the file text still contains `speedrun_id = "4pd0n31e"` right after the name line
}
#[test]
fn a_game_without_a_link_writes_no_speedrun_id() {
    // Game::create(&config, "Portal"); file text does not contain "speedrun_id"
}
#[test]
fn linking_names_the_game_as_on_speedrun_com_and_keeps_its_slug() {
    let (_dir, config) = config();
    Game::create(&config, "GTA SA DE").unwrap();
    let link = SpeedrunLink { id: "yo1yv1q5".into(), name: "Grand Theft Auto: San Andreas".into() };
    let game = Game::set_link(&config, "gta-sa-de", Some(&link)).unwrap();
    assert_eq!((game.slug.as_str(), game.name.as_str(), game.speedrun_id.as_deref()),
               ("gta-sa-de", "Grand Theft Auto: San Andreas", Some("yo1yv1q5")));
    assert_eq!(Game::load(&config, "gta-sa-de").unwrap(), game);
}
#[test]
fn unlinking_removes_the_id_and_keeps_the_name() {
    // link, then set_link(None) -> speedrun_id None, name unchanged, file has no "speedrun_id"
}
#[test]
fn the_list_tells_which_games_are_linked() {
    // two games, one linked -> Game::list gives speedrun_id Some/None accordingly
}
```

- [ ] **Step 2:** Run `cargo test config::games`. Expected: FAIL.
- [ ] **Step 3: Implement.** In `GameFile`, add `#[serde(default, skip_serializing_if = "Option::is_none")] speedrun_id: Option<String>`
  directly after `name`. The field order in the struct is the order in
  the file.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(config): store a game's speedrun.com id`.

### Task 4: Game dialog search, list and link

**Files:**
- Modify: `src/ui/components/game_dialog.rs` (state, rendering, tests)
- Modify: `src/ui/components/mod.rs` (export `GameTarget`)

**Interfaces:**
- Consumes: `GameSearch`, `SearchAnswer`, `FakeSearch`, `SpeedrunGame`,
  `SpeedrunLink`, `clear_match` (Tasks 1 and 2), and
  `GameSummary.speedrun_id` (Task 3).
- Produces (replacing today's `GameChoice` enum):
  - `pub enum GameTarget { Existing(String), New(String) }` (by slug, and by
    name)
  - `pub struct GameChoice { pub game: GameTarget, pub link: Option<SpeedrunLink> }`
  - `GameDialog::new(file_name: &str, games: Vec<GameSummary>, search: Box<dyn GameSearch>) -> Self`
  - `GameDialog::change(file_name: &str, games: Vec<GameSummary>, current: &GameSummary, search: Box<dyn GameSearch>) -> Self`
  - `pub fn take_failure(&mut self) -> Option<String>`: the first failed
    answer's error, returned once per dialog.
  - `GameDialogAction::Use(GameChoice)` and `Cancel`, and `game_dialog(ctx, dialog)`,
    keep their signatures.

**State.** `GameDialog` keeps `kind`, `file_name`, `games`, `name`,
`selected` and `focus_pending`, and adds:
`link: Option<SpeedrunLink>`, `search: Box<dyn GameSearch>`,
`results: HashMap<String, Vec<SpeedrunGame>>` (by trimmed name),
`due: Option<(String, f64)>` (name, input time when it is sent),
`list_open: bool`, `highlight: Option<usize>`, `may_link_on_own: bool`,
`declined: HashSet<String>`, `failure: Option<String>` and
`failure_taken: bool`. Rendering stays a thin layer over methods that the
tests drive directly, taking `now: f64` (egui's `input.time`):

- `typed(&mut self, now)`: the field changed. Clears `link`, runs
  `follow_name`, and takes the selected game's link if it has one (spec
  §6.8). Sets `may_link_on_own = false`, closes the list and clears the
  highlight. If nothing is linked and the name isn't blank, sets `due` to
  the name and `now + 0.3`.
- `search_now(&mut self, now)`: on open (when not linked) and from
  `select`. Sets `may_link_on_own = true` and makes the search due at `now`.
- `tick(&mut self, now)`, every frame: sends `due` once it is reached.
  Results already in `results` are used without a new request. Then it
  drains `search.poll()` into `results`, keeping the first `Err` in
  `failure`. When results for the current trimmed name arrive, and the
  name isn't linked or declined: with `may_link_on_own` and a
  `clear_match`, it links (`link`, and `name` = the result's name).
  Otherwise it opens the list. An error opens the list too, holding only
  the last row.
- `pick(&mut self, index)`: links that result, fills `name`, and closes the
  list. `selected` doesn't change.
- `pick_typed(&mut self)`: the last row. Closes the list, adds the trimmed
  name to `declined`, and leaves `link` as `None`.
- `change_link(&mut self, now)`: clears `link` and opens the list for the
  name, searching it if it has no results yet. It doesn't link on its own.
- `select(index)` (existing): also sets `link` from the summary. With no
  link, it calls `search_now`.
- `key(&mut self, key: Key) -> bool`: while the list is open, Down and Up
  move `highlight` (None → 0, clamped; the last row's index is
  `results.len()`), Enter picks the highlight, and Escape closes the list.
  Returns whether it used the key.
- `choice()`: `link` is `Some(l)` and a game in `games` has
  `speedrun_id == l.id` → `Existing(that slug)`. Otherwise `selected` →
  `Existing(slug)`, otherwise a non-blank name → `New(name)`. The `link`
  rides along in every case.

**Rendering** follows the Superdesign component from Task 0. The list is
an `egui::Area` at `Order::Tooltip`, anchored under the field and as wide
as the field. Results go in a `ScrollArea` of max height 5 × 32 px, and the
last row sits under it, outside the scroll. Each row shows the name
(body 13), the year (mono 11, `TEXT_MUTED`) and `SET UP` (mono 11) when a
game in `games` has its id. The linked line shows under the field when
`link` is `Some`: a 8 px `STATUS_OK` dot, "Linked to speedrun.com" (body
12, `TEXT_SECONDARY`) and `Button::secondary("Change")` at 24 px. While
`due` is set or an answer is awaited, call
`ctx.request_repaint_after(Duration::from_millis(100))`. Feed keys to
`key()` with `ui.input_mut(|i| i.consume_key(..))` before the field and
the modal see them. Then Escape on an open list doesn't cancel the dialog,
and an Enter that picked a row doesn't also count as **Use this game**.

- [ ] **Step 1: Write the failing tests** (state level, with `FakeSearch`;
  `t(0.0)` stands for the time argument):

```rust
#[test] fn opening_searches_the_name_from_the_file_at_once()
//   new("portal.wasm", vec![], fake); tick(0.0) -> fake.requests == ["portal"]
#[test] fn a_clear_match_on_open_links_and_renames()
//   answer "portal" with [Portal 4pd0n31e, Portals]; tick -> name == "Portal",
//   choice() == GameChoice { game: New("Portal"), link: Some(Portal link) }, list closed
#[test] fn no_clear_match_on_open_opens_the_list()
//   "gta_sa_de.wasm"; answer "gta sa de" with [] -> list open, link None
#[test] fn typing_waits_300_ms_and_never_links_on_its_own()
//   typed at 1.0 with name "portal"; tick(1.2) -> no request; tick(1.31) -> ["portal"];
//   answer with exact Portal -> link None, list open, name still "portal"
#[test] fn picking_a_result_links_the_selected_game_without_changing_it()
//   games [gta-sa-de "GTA SA DE" unlinked]; select(0); answer with [] ;
//   pick a result -> choice() == { game: Existing("gta-sa-de"), link: Some(..) }
#[test] fn a_result_already_linked_uses_that_game()
//   games [portal linked 4pd0n31e]; pick Portal result -> game Existing("portal")
#[test] fn typing_removes_the_link_and_selecting_a_linked_game_brings_its_link()
#[test] fn the_last_row_keeps_the_typed_name_unlinked_and_stops_linking_it()
//   pick_typed(); then a later answer for that name doesn't link or reopen the list
#[test] fn change_unlinks_and_opens_the_list_without_linking_on_its_own()
#[test] fn change_game_on_a_linked_game_does_not_search()
//   change(.., current linked, fake); tick -> fake.requests empty; link Some
#[test] fn answers_for_an_old_name_are_ignored()
//   answer for "portal" after the name became "portal 2" -> list closed, link None
#[test] fn results_are_reused_for_a_name_searched_before()
#[test] fn offline_shows_only_the_last_row_and_reports_once()
//   two Err answers -> list open with no results; take_failure() Some then None
#[test] fn arrows_and_enter_pick_and_escape_closes_the_list()
//   key(ArrowDown) -> highlight 0; key(Enter) -> picked, returns true; with the list closed key(Enter) -> false
```

Add one headless test that runs `game_dialog` in a `Context` with the list
open: a Down press, then Enter in the next frame, returns `None` (no
`Use`), and the dialog is linked. Follow the `RawInput` pattern in
`src/ui/preferences_tab.rs`. Keep the existing tests, rewritten for the
new `GameChoice`.

- [ ] **Step 2:** Run `cargo test game_dialog`. Expected: FAIL.
- [ ] **Step 3: Implement** the state methods, then the rendering.
- [ ] **Step 4:** Run `cargo test`. Expected: PASS. Then run
  `cargo clippy --all-targets --locked -- -D warnings`.
- [ ] **Step 5: Commit** `feat(ui): suggest speedrun.com games in the game dialog`.

### Task 5: App wiring

**Files:**
- Modify: `src/ui/mod.rs` (`BridgeApp` field, `load`, `change_game`,
  `game_dialog`, tests)

**Interfaces:**
- Consumes: everything above.
- Produces: `BridgeApp.new_search: fn() -> Box<dyn GameSearch>`, set in `new`
  to `speedrun::live`, or to a function returning `Box::new(Offline)`
  under `cfg(test)`. Tests may replace it.

- [ ] **Step 1: Write the failing tests** in `ui/mod.rs`'s test module.
  Open the dialog through `app.load(path)` and `app.change_game()` with
  `new_search` set to a `FakeSearch` factory, or set `app.dialog` directly
  with a dialog whose state is prepared. Then call the extracted
  `apply_game_choice(&mut self, dialog: &GameDialog, path: PathBuf, choice: GameChoice)`
  (split out of `game_dialog`, so it can be tested without a frame).

```rust
#[test] fn using_a_new_linked_game_sets_it_up_named_and_linked()
//   New("Portal") + link Portal -> games/portal.toml has name "Portal", speedrun_id
#[test] fn linking_an_existing_game_renames_it_and_logs_it()
//   existing "gta" ("GTA"), Existing("gta") + link GTA SA -> file name = speedrun name,
//   slug "gta", splitters.toml still maps the wasm to "gta", log has an App line containing "speedrun.com"
#[test] fn using_a_linked_game_without_a_link_unlinks_it()
#[test] fn a_failed_search_is_logged_once_in_app()
//   dialog whose failure is set; two frames of game_dialog -> exactly one App log line containing "speedrun.com"
```

- [ ] **Step 2:** Run `cargo test ui::tests`. Expected: FAIL.
- [ ] **Step 3: Implement.** Pass `(self.new_search)()` to every
  `GameDialog::new` and `GameDialog::change`. In `game_dialog`, before the
  action, call `dialog.take_failure()` and log it as
  `Category::App, "Couldn't search speedrun.com: {error}"`.
  `apply_game_choice` resolves the game. `New` creates it with
  `link.name` if linked, otherwise with the typed name. `Existing` reads
  it. Then, when the game's `speedrun_id` or name differs from the link
  (including `Some` → `None`), it calls `Game::set_link` and logs
  `Category::App`: "Linked {name} to speedrun.com" or
  "Unlinked {name} from speedrun.com". A `set_link` error is logged as
  `Category::Error`, and the game is used unchanged. Everything after that
  (`remember_game`, load or change) stays as it is.
- [ ] **Step 4:** Run `cargo test`, `cargo clippy --all-targets --locked -- -D warnings`
  and `cargo fmt --check`. Expected: all pass.
- [ ] **Step 5: Commit** `feat(ui): link games to speedrun.com when they are used`.

### Task 6: Wiki

**Files:**
- Modify: `docs/wiki/Auto-Splitters-and-Settings.md` ("Which game it's for"),
  `docs/wiki/Getting-Started.md` (step 3), `docs/wiki/Troubleshooting.md`
  (no suggestions: offline or speedrun.com unreachable, and what still
  works), `docs/wiki/Logs-and-Files.md` (if it lists App & runtime
  contents or the game file's keys)

No full release exists yet, so the pages are edited directly, not Upcoming
Changes (spec §15.3).

- [ ] **Step 1:** Write it in the voice of `assets/brand/BRAND.md`.
  Describe the suggestions, the linking on its own, **Change**, the last
  row, renaming, unlinking, and what is sent to speedrun.com and when (spec
  §10).
- [ ] **Step 2:** Run `scripts/wiki/check.sh`. Expected: no errors.
- [ ] **Step 3: Commit** `docs(wiki): document speedrun.com game suggestions`.
  The pull request says screenshot S3 (`game-dialog.png`) needs retaking.
