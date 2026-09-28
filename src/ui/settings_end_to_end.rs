//! End to end for settings (spec §6.4, §7.2, §12): the whole app, with the
//! real Runner, loads the test auto splitter that publishes every kind of
//! widget and stores values of every type itself, and is driven through its
//! window, drawn headless: the settings show, an edit is saved, and every
//! value survives in the game's file and a reload.

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use eframe::egui::{
    self, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, accesskit, pos2, vec2,
};
use livesplit_auto_splitting::settings::WidgetKind;
use toml::{Table, Value};

use super::{BridgeApp, Tab, settings};
use crate::config::{AppSettings, Config, Game, Splitters};

/// Long enough for a slow CI machine to compile the auto splitter.
const TIMEOUT: Duration = Duration::from_secs(30);

const EVERY_KIND: &str = include_str!("../runner/test_auto_splitters/every_kind.wat");

/// A widget on screen, as the accessibility tree describes it.
#[derive(Debug, Clone)]
struct Node {
    label: String,
    rect: Rect,
    toggled: Option<bool>,
}

struct Harness {
    _dir: tempfile::TempDir,
    config: Config,
    path: PathBuf,
    app: BridgeApp,
}

impl Harness {
    /// The app, with the auto splitter set up for the game "GTA", whose
    /// file already has another auto splitter's value and `gym` off.
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::at(dir.path().join("config"));
        let path = dir.path().join("every_kind.wasm");
        std::fs::write(&path, wat::parse_str(EVERY_KIND).unwrap()).unwrap();
        let game = Game::create(&config, "GTA").unwrap();
        let saved: Table = "other_splitter = 3\ngym = false\n".parse().unwrap();
        Game::save_settings(&config, &game.slug, &saved).unwrap();
        let mut splitters = Splitters::default();
        splitters.associate(&path, &game.slug).unwrap();
        splitters.save(&config).unwrap();

        let ctx = egui::Context::default();
        super::theme::install(&ctx);
        ctx.enable_accesskit();
        let loaded = Some(AppSettings::load(&config));
        let mut app = BridgeApp::new(&ctx, Some(config.clone()), loaded);
        app.tab = Tab::Settings;
        Self {
            _dir: dir,
            config,
            path,
            app,
        }
    }

    /// Draws one frame of the whole window with `events`, returning what is
    /// on screen.
    fn frame(&mut self, events: Vec<Event>) -> Vec<Node> {
        let ctx = self.app.ctx.clone();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 1400.0))),
            events,
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(&mut self.app, ui, &mut frame);
        });
        // Nothing draws the frame, so its textures are dropped.
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .map(|update| update.nodes)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(_, node): (accesskit::NodeId, accesskit::Node)| {
                let bounds = node.bounds()?;
                Some(Node {
                    label: node.label().or(node.value())?.to_owned(),
                    rect: Rect::from_min_max(
                        pos2(bounds.x0 as f32, bounds.y0 as f32),
                        pos2(bounds.x1 as f32, bounds.y1 as f32),
                    ),
                    toggled: node
                        .toggled()
                        .map(|toggled| toggled == accesskit::Toggled::True),
                })
            })
            .collect()
    }

    /// Draws frames until `done` holds, failing the test on timeout.
    fn until(&mut self, what: &str, done: impl Fn(&BridgeApp, &[Node]) -> bool) -> Vec<Node> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let nodes = self.frame(Vec::new());
            if done(&self.app, &nodes) {
                return nodes;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Clicks the widget labelled `label`.
    fn click(&mut self, label: &str) {
        let nodes = self.frame(Vec::new());
        let position = find(&nodes, label).rect.center();
        let button = |pressed| Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(position), button(true)]);
        self.frame(vec![button(false)]);
    }

    fn saved(&self) -> Table {
        Game::load(&self.config, "gta").unwrap().settings
    }
}

fn find<'a>(nodes: &'a [Node], label: &str) -> &'a Node {
    nodes
        .iter()
        .find(|node| node.label == label)
        .unwrap_or_else(|| panic!("no {label:?} in {nodes:#?}"))
}

fn has(nodes: &[Node], label: &str) -> bool {
    nodes.iter().any(|node| node.label == label)
}

/// Whether the auto splitter has published its widgets and stored its
/// values.
fn ready(app: &BridgeApp, nodes: &[Node]) -> bool {
    has(nodes, "Gym Moves") && app.settings.live().contains_key("best_route_version")
}

#[test]
fn every_kind_of_setting_shows_saves_and_survives_a_reload() {
    let mut harness = Harness::new();
    let path = harness.path.clone();
    harness.app.load(path);
    let nodes = harness.until("the settings", ready);

    // Every kind of widget, with the saved value, the default, or what the
    // auto splitter stored.
    for label in [
        "GENERAL",
        "Files",
        "Collectibles",
        "Horseshoes by area",
        "Category",
        "Any%",
        "Split on level",
        "Route",
        "No file selected",
        "Browse…",
        "Images · JSON application files · Rust · Images · Plain text files",
    ] {
        assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
    }
    assert_eq!(find(&nodes, "Gym Moves").toggled, Some(false));
    assert!(has(&nodes, "chapter_2"), "{nodes:#?}");
    assert!(has(&nodes, "Default: prologue"), "{nodes:#?}");
    // Only in developer mode.
    assert!(!has(&nodes, "best_route_version"));

    // The file dialog gets LiveSplit's filters.
    let route = harness
        .app
        .settings
        .widgets()
        .iter()
        .find(|widget| &*widget.key == "route")
        .cloned()
        .unwrap();
    let WidgetKind::FileSelect { filters } = &route.kind else {
        panic!("route isn't a file selection");
    };
    let names: Vec<_> = settings::file_filters(filters)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let mut expected = vec![
        "Images",
        "JSON application files",
        "Rust",
        "Images",
        "Plain text files",
    ];
    if cfg!(not(target_os = "macos")) {
        expected.push(settings::ALL_FILES);
    }
    assert_eq!(names, expected);

    // What the auto splitter stored is written to the game's file without
    // saving, with what was there.
    let written = harness.until("the stored values to be written", |app, _| {
        app.settings_write.is_none()
    });
    drop(written);
    let saved = harness.saved();
    assert_eq!(saved["other_splitter"].as_integer(), Some(3));
    assert_eq!(saved["best_route_version"].as_integer(), Some(7));
    assert_eq!(saved["level"].as_str(), Some("chapter_2"));

    // Edits wait for Save.
    harness.click("Gym Moves");
    harness.click("Use default");
    let nodes = harness.frame(Vec::new());
    assert!(has(&nodes, "● Unsaved changes"), "{nodes:#?}");
    assert_eq!(harness.saved()["gym"].as_bool(), Some(false));
    harness.click("Save");
    harness.until("the edits to reach the auto splitter", |app, _| {
        let live = app.settings.live();
        live.get("gym") == Some(&Value::Boolean(true))
            && live.get("level").and_then(Value::as_str) == Some("prologue")
    });

    // Every value, of every type, is in the file: the edits, what the auto
    // splitter stored, and the other auto splitter's value.
    let saved = harness.saved();
    let expected: Table = "other_splitter = 3\n\
         gym = true\n\
         best_route_version = 7\n\
         igt_offset = 1.25\n\
         level = \"prologue\"\n\
         completed_missions = [\"intro\", true]\n\
         [last_run]\n\
         splits = 42\n"
        .parse()
        .unwrap();
    for (key, value) in &expected {
        assert_eq!(saved.get(key), Some(value), "{key} in {saved:#?}");
    }
    assert_eq!(saved.len(), expected.len(), "{saved:#?}");

    // Reloading starts from the file. The auto splitter stores its level
    // again: the last writer wins, as in LiveSplit.
    harness.app.reload();
    let nodes = harness.until("the reloaded settings", |app, nodes| {
        ready(app, nodes) && app.settings.live().get("level") != Some(&Value::from("prologue"))
    });
    assert_eq!(find(&nodes, "Gym Moves").toggled, Some(true));
    assert_eq!(
        harness.app.settings.live()["last_run"]["splits"].as_integer(),
        Some(42)
    );

    // Developer mode shows the whole map.
    harness.app.developer_mode = true;
    harness.app.save_developer_mode();
    let nodes = harness.frame(Vec::new());
    for label in [
        "SETTINGS MAP · 7 VALUES",
        "DEVELOPER",
        "best_route_version",
        "other_splitter",
        "last_run",
    ] {
        assert!(has(&nodes, label), "no {label:?} in {nodes:#?}");
    }
    assert!(harness.app.log_filters.auto_splitter);
}

#[test]
fn a_cleared_file_is_removed_from_the_games_file() {
    let mut harness = Harness::new();
    let path = harness.path.clone();
    harness.app.load(path);
    harness.until("the settings", ready);

    let route = super::wasi_path::from_native(harness.path.as_path()).unwrap();
    let widget = harness
        .app
        .settings
        .widgets()
        .iter()
        .find(|widget| &*widget.key == "route")
        .cloned()
        .unwrap();
    harness
        .app
        .settings
        .set(&widget, Some(Value::String(route.into())));
    harness.click("Save");
    harness.until("the file to be saved", |app, _| {
        app.settings.live().contains_key("route")
    });
    assert!(harness.saved().contains_key("route"));

    harness.click("Clear");
    harness.click("Save");
    harness.until("the file to be cleared", |app, _| {
        !app.settings.live().contains_key("route")
    });
    assert!(!harness.saved().contains_key("route"));
    // Everything else is kept.
    assert_eq!(harness.saved()["best_route_version"].as_integer(), Some(7));
}
