//! Runs the test auto splitters in `test_auto_splitters/` in the Runner.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::{Duration, Instant},
};

use livesplit_auto_splitting::{
    TimerState,
    settings::{Map, Value},
    time,
};

use super::*;

const SCHEDULE: &str = include_str!("test_auto_splitters/schedule.wat");
const SLOW: &str = include_str!("test_auto_splitters/slow.wat");
const HANG: &str = include_str!("test_auto_splitters/hang.wat");
const TRAP: &str = include_str!("test_auto_splitters/trap.wat");
const ATTACH: &str = include_str!("test_auto_splitters/attach.wat");
const SETTING: &str = include_str!("test_auto_splitters/setting.wat");
const WIDGETS: &str = include_str!("test_auto_splitters/widgets.wat");

/// Long enough for a slow CI machine to compile an auto splitter.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Records the actions sent to the timer.
#[derive(Default)]
struct RecordingLink {
    actions: Mutex<Vec<TimerAction>>,
}

impl TimerLink for RecordingLink {
    fn state(&self) -> TimerState {
        TimerState::NotRunning
    }

    fn current_split_index(&self) -> Option<usize> {
        None
    }

    fn segment_splitted(&self, _index: usize) -> Option<bool> {
        None
    }

    fn send(&self, action: TimerAction) {
        self.actions.lock().unwrap().push(action);
    }
}

struct Harness {
    runner: Runner,
    events: Receiver<RunnerEvent>,
    link: Arc<RecordingLink>,
    dir: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        let link = Arc::new(RecordingLink::default());
        let (runner, events) = Runner::new(link.clone(), || {});
        Self {
            runner,
            events,
            link,
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// Writes an auto splitter, from WebAssembly text, to `name` in the
    /// test's folder.
    fn write(&self, name: &str, wat: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        write_wasm(&path, wat);
        path
    }

    /// Collects events until one matches, failing the test on timeout.
    fn wait_for(&self, matches: impl Fn(&RunnerEvent) -> bool) -> Vec<RunnerEvent> {
        let deadline = Instant::now() + TIMEOUT;
        let mut seen = Vec::new();
        loop {
            match self
                .events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            {
                Ok(event) => {
                    let done = matches(&event);
                    seen.push(event);
                    if done {
                        return seen;
                    }
                }
                Err(RecvTimeoutError::Timeout) => panic!("timed out; events so far: {seen:#?}"),
                Err(RecvTimeoutError::Disconnected) => panic!("events closed: {seen:#?}"),
            }
        }
    }

    fn load(&self, path: &Path) -> Vec<RunnerEvent> {
        self.runner.load(path.to_owned(), Map::new());
        self.wait_for(|event| matches!(event, RunnerEvent::Loaded { path: p, .. } if p == path))
    }

    /// Collects every event that arrives within `duration`.
    fn collect_for(&self, duration: Duration) -> Vec<RunnerEvent> {
        let deadline = Instant::now() + duration;
        let mut seen = Vec::new();
        while let Ok(event) = self
            .events
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            seen.push(event);
        }
        seen
    }
}

fn write_wasm(path: &Path, wat: &str) {
    fs::write(path, wat::parse_str(wat).unwrap()).unwrap();
}

fn log(message: &str) -> RunnerEvent {
    RunnerEvent::AutoSplitterLog(message.to_owned())
}

/// An auto splitter that logs `message` on every tick.
fn logging(message: &str) -> String {
    format!(
        r#"(module
          (import "env" "runtime_print_message" (func $print (param i32 i32)))
          (memory (export "memory") 1)
          (data (i32.const 0) "{message}")
          (func (export "update") (call $print (i32.const 0) (i32.const {}))))"#,
        message.len()
    )
}

fn is_crash(event: &RunnerEvent) -> bool {
    matches!(event, RunnerEvent::Crashed { .. })
}

fn is_attached(event: &RunnerEvent) -> bool {
    matches!(event, RunnerEvent::GameAttached { .. })
}

/// The tick rate upstream starts every auto splitter at: 120 ticks a second.
fn default_tick_rate() -> Duration {
    Duration::from_secs_f64(1.0 / 120.0)
}

#[test]
fn runs_the_schedule_and_reports_actions_and_logs_in_order() {
    let harness = Harness::new();
    let path = harness.write("schedule.wasm", SCHEDULE);
    harness.load(&path);
    let events = harness.wait_for(|event| *event == log("finished"));

    let observed: Vec<_> = events
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                RunnerEvent::AutoSplitterLog(_) | RunnerEvent::TimerAction(_)
            )
        })
        .collect();
    let game_time = time::Duration::new(1, 500_000_000);
    assert_eq!(
        observed,
        [
            log("started"),
            RunnerEvent::TimerAction(TimerAction::Start),
            RunnerEvent::TimerAction(TimerAction::Split),
            RunnerEvent::TimerAction(TimerAction::SetGameTime(game_time)),
            RunnerEvent::TimerAction(TimerAction::Split),
            log("finished"),
        ]
    );
    // The last action is sent right after the last event, so give it a
    // moment before checking what reached the timer.
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        *harness.link.actions.lock().unwrap(),
        [
            TimerAction::Start,
            TimerAction::Split,
            TimerAction::SetGameTime(game_time),
            TimerAction::Split,
            TimerAction::Reset,
        ]
    );
    assert_eq!(harness.runner.loaded_path(), Some(path));
}

#[test]
fn loaded_reports_the_default_tick_rate() {
    let harness = Harness::new();
    let path = harness.write("splitter.wasm", &logging("tick"));
    let events = harness.load(&path);
    assert_eq!(
        events.last(),
        Some(&RunnerEvent::Loaded {
            path,
            tick_rate: default_tick_rate()
        })
    );
}

#[test]
fn a_tick_rate_set_while_ticking_follows_the_default_one() {
    // Upstream runs all of an auto splitter's code, including `_start`, in
    // ticks, so a requested rate always arrives after the load.
    let harness = Harness::new();
    let path = harness.write("slow.wasm", SLOW);
    let events = harness.load(&path);
    assert_eq!(
        events.last(),
        Some(&RunnerEvent::Loaded {
            path,
            tick_rate: default_tick_rate()
        })
    );
    harness.wait_for(|event| {
        *event == RunnerEvent::TickRateChanged(Duration::from_secs_f64(1.0 / 10.0))
    });
}

#[test]
fn ticks_at_the_rate_the_auto_splitter_requests() {
    let harness = Harness::new();
    let path = harness.write("slow.wasm", SLOW);
    harness.load(&path);
    harness.wait_for(|event| {
        *event == RunnerEvent::TickRateChanged(Duration::from_secs_f64(1.0 / 10.0))
    });

    let ticks = harness
        .collect_for(Duration::from_secs(1))
        .into_iter()
        .filter(|event| *event == log("tick"))
        .count();
    // 10 ticks a second were requested; the default is 120. Loose bounds, so
    // a busy CI machine doesn't fail the test.
    assert!((3..=20).contains(&ticks), "{ticks} ticks in one second");
}

#[test]
fn reload_picks_up_a_rebuilt_file() {
    let harness = Harness::new();
    let path = harness.write("splitter.wasm", &logging("version 1"));
    harness.load(&path);
    harness.wait_for(|event| *event == log("version 1"));

    write_wasm(&path, &logging("version 2"));
    harness.runner.reload(Map::new());
    harness.wait_for(|event| matches!(event, RunnerEvent::Loaded { .. }));
    harness.wait_for(|event| *event == log("version 2"));
}

#[test]
fn a_file_that_fails_to_load_leaves_the_previous_auto_splitter_running() {
    let harness = Harness::new();
    let slow = harness.write("slow.wasm", SLOW);
    harness.load(&slow);
    harness.wait_for(|event| *event == log("tick"));

    let broken = harness.dir.path().join("broken.wasm");
    fs::write(&broken, b"not a wasm file").unwrap();
    harness.runner.load(broken.clone(), Map::new());
    harness
        .wait_for(|event| matches!(event, RunnerEvent::LoadFailed { path, .. } if *path == broken));

    harness.wait_for(|event| *event == log("tick"));
    assert_eq!(harness.runner.loaded_path(), Some(slow));
}

#[test]
fn a_missing_file_or_update_function_fails_to_load() {
    let harness = Harness::new();
    let missing = harness.dir.path().join("missing.wasm");
    harness.runner.load(missing, Map::new());
    harness.wait_for(|event| matches!(event, RunnerEvent::LoadFailed { .. }));

    let no_update = harness.write("no-update.wasm", r#"(module (memory (export "memory") 1))"#);
    harness.runner.load(no_update, Map::new());
    let events = harness.wait_for(|event| matches!(event, RunnerEvent::LoadFailed { .. }));
    let RunnerEvent::LoadFailed { error, .. } = events.last().unwrap() else {
        unreachable!()
    };
    assert!(error.contains("update"), "{error}");
    assert_eq!(harness.runner.loaded_path(), None);
}

#[test]
fn reload_interrupts_a_hung_auto_splitter() {
    let harness = Harness::new();
    let path = harness.write("splitter.wasm", HANG);
    harness.load(&path);
    // Let it get stuck in its first tick.
    std::thread::sleep(Duration::from_millis(200));

    write_wasm(&path, &logging("unstuck"));
    harness.runner.reload(Map::new());
    let events = harness.wait_for(|event| *event == log("unstuck"));
    assert!(!events.iter().any(is_crash), "{events:#?}");
}

#[test]
fn loading_another_file_interrupts_a_hung_auto_splitter() {
    let harness = Harness::new();
    harness.load(&harness.write("hang.wasm", HANG));
    std::thread::sleep(Duration::from_millis(200));

    harness.load(&harness.write("slow.wasm", SLOW));
    harness.wait_for(|event| *event == log("tick"));
}

#[test]
fn unload_interrupts_a_hung_auto_splitter() {
    let harness = Harness::new();
    harness.load(&harness.write("hang.wasm", HANG));
    std::thread::sleep(Duration::from_millis(200));

    harness.runner.unload();
    harness.wait_for(|event| *event == RunnerEvent::Unloaded);
    assert_eq!(harness.runner.loaded_path(), None);

    // The Runner thread is free again: it runs the next auto splitter.
    harness.load(&harness.write("slow.wasm", SLOW));
    let events = harness.wait_for(|event| *event == log("tick"));
    assert!(!events.iter().any(is_crash), "{events:#?}");
}

#[test]
fn dropping_the_runner_interrupts_a_hung_auto_splitter() {
    let harness = Harness::new();
    harness.load(&harness.write("hang.wasm", HANG));
    std::thread::sleep(Duration::from_millis(200));

    let started = Instant::now();
    drop(harness.runner);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_crash_is_reported_and_reload_starts_it_again() {
    let harness = Harness::new();
    let path = harness.write("splitter.wasm", TRAP);
    harness.load(&path);
    harness.wait_for(is_crash);
    // The file is still loaded, so Reload can start it again.
    assert_eq!(harness.runner.loaded_path(), Some(path.clone()));

    write_wasm(&path, &logging("fixed"));
    harness.runner.reload(Map::new());
    harness.wait_for(|event| *event == log("fixed"));
}

#[test]
fn reports_attaching_to_and_detaching_from_the_game() {
    let harness = Harness::new();
    // The test process stands in for the game.
    let wat = ATTACH.replace("PID", &std::process::id().to_string());
    harness.load(&harness.write("attach.wasm", &wat));
    let events = harness.wait_for(|event| *event == RunnerEvent::GameDetached);
    let attached: Vec<_> = events.iter().filter(|event| is_attached(event)).collect();
    let [RunnerEvent::GameAttached { process }] = attached[..] else {
        panic!("expected one attach: {events:#?}");
    };

    // The name comes from the path the OS reports for the process, and
    // `current_exe` asks the OS the same thing, so the file names match.
    // Compared ignoring case, as Windows paths are case-insensitive and the
    // two APIs needn't agree on case.
    let exe = std::env::current_exe().unwrap();
    let expected = exe.file_name().unwrap().to_str().unwrap();
    let process = process.as_deref().expect("no process name");
    assert!(
        process.eq_ignore_ascii_case(expected),
        "{process} != {expected}"
    );
}

#[test]
fn unloading_an_attached_auto_splitter_reports_the_game_detached() {
    let harness = Harness::new();
    let wat = format!(
        r#"(module
          (import "env" "process_attach_by_pid" (func $attach (param i64) (result i64)))
          (memory (export "memory") 1)
          (global $attached (mut i32) (i32.const 0))
          (func (export "update")
            (if (i32.eqz (global.get $attached))
              (then
                (drop (call $attach (i64.const {})))
                (global.set $attached (i32.const 1))))))"#,
        std::process::id()
    );
    harness.load(&harness.write("attach.wasm", &wat));
    harness.wait_for(is_attached);

    harness.runner.unload();
    harness.wait_for(|event| *event == RunnerEvent::GameDetached);
}

/// A test double that follows the auto splitter's own actions, like a
/// connected timer would, so state queries can be tested before the Server
/// exists.
#[derive(Default)]
struct StateTrackingLink {
    /// The split index while running.
    split_index: Mutex<Option<usize>>,
}

impl TimerLink for StateTrackingLink {
    fn state(&self) -> TimerState {
        match *self.split_index.lock().unwrap() {
            Some(_) => TimerState::Running,
            None => TimerState::NotRunning,
        }
    }

    fn current_split_index(&self) -> Option<usize> {
        *self.split_index.lock().unwrap()
    }

    fn segment_splitted(&self, _index: usize) -> Option<bool> {
        None
    }

    fn send(&self, action: TimerAction) {
        let mut split_index = self.split_index.lock().unwrap();
        match action {
            TimerAction::Start => *split_index = split_index.or(Some(0)),
            TimerAction::Split => *split_index = split_index.map(|index| index + 1),
            TimerAction::Reset => *split_index = None,
            _ => {}
        }
    }
}

/// Collects timer actions until a reset, failing the test on timeout.
fn actions_until_reset(events: &Receiver<RunnerEvent>) -> Vec<TimerAction> {
    let deadline = Instant::now() + TIMEOUT;
    let mut actions = Vec::new();
    while actions.last() != Some(&TimerAction::Reset) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match events.recv_timeout(remaining) {
            Ok(RunnerEvent::TimerAction(action)) => actions.push(action),
            Ok(_) => {}
            Err(_) => panic!(
                "timed out after {} actions; the first ones: {:?}",
                actions.len(),
                &actions[..actions.len().min(10)]
            ),
        }
    }
    actions
}

#[test]
fn state_queries_go_through_the_timer_link() {
    // Real auto splitters check the state before acting: this one starts when
    // the timer isn't running and splits while it is, so it only splits if
    // the link's answers reach it.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state-aware.wasm");
    write_wasm(&path, include_str!("test_auto_splitters/state_aware.wat"));
    let (runner, events) = Runner::new(Arc::new(StateTrackingLink::default()), || {});
    runner.load(path, Map::new());

    assert_eq!(
        actions_until_reset(&events),
        [
            TimerAction::Start,
            TimerAction::Split,
            TimerAction::Split,
            TimerAction::Split,
            TimerAction::Reset,
        ]
    );
}

#[test]
fn with_no_timer_an_auto_splitter_that_checks_the_state_only_tries_to_start() {
    // Spec §5.2: with no timer connected, the state is "not running", so a
    // state-checking auto splitter keeps trying to start and never splits.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state-aware.wasm");
    write_wasm(&path, include_str!("test_auto_splitters/state_aware.wat"));
    let (runner, events) = Runner::new(Arc::new(NoTimer), || {});
    runner.load(path, Map::new());

    let deadline = Instant::now() + Duration::from_millis(500);
    let mut actions = Vec::new();
    while let Ok(event) = events.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        if let RunnerEvent::TimerAction(action) = event {
            actions.push(action);
        }
    }
    assert!(!actions.is_empty());
    assert!(
        actions.iter().all(|action| *action == TimerAction::Start),
        "{actions:?}"
    );
}

fn gym(on: bool) -> Map {
    let mut settings = Map::new();
    settings.insert("gym".into(), Value::Bool(on));
    settings
}

#[test]
fn starts_with_the_settings_it_is_given() {
    let harness = Harness::new();
    let path = harness.write("setting.wasm", SETTING);
    harness.runner.load(path.clone(), gym(true));
    harness.wait_for(|event| *event == log("on"));

    harness.runner.reload(Map::new());
    harness.wait_for(|event| matches!(event, RunnerEvent::Loaded { .. }));
    harness.wait_for(|event| *event == log("unset"));
}

#[test]
fn new_settings_reach_the_running_auto_splitter() {
    let harness = Harness::new();
    let path = harness.write("setting.wasm", SETTING);
    harness.runner.load(path, gym(false));
    harness.wait_for(|event| *event == log("off"));

    harness.runner.set_settings(gym(true));
    let events = harness.wait_for(|event| *event == log("on"));
    assert!(
        !events.iter().any(|event| matches!(
            event,
            RunnerEvent::Loaded { .. } | RunnerEvent::GameDetached
        )),
        "{events:#?}"
    );
}

#[test]
fn reports_the_settings_widgets_the_auto_splitter_publishes() {
    let harness = Harness::new();
    let path = harness.write("setting.wasm", SETTING);
    harness.load(&path);
    let events = harness.wait_for(|event| matches!(event, RunnerEvent::SettingsWidgets(_)));
    let Some(RunnerEvent::SettingsWidgets(widgets)) = events.last() else {
        unreachable!()
    };
    let keys: Vec<_> = widgets.0.iter().map(|widget| &*widget.key).collect();
    assert_eq!(keys, ["gym"]);

    // Published once: the same widgets aren't reported on every tick.
    harness.wait_for(|event| *event == log("unset"));
    let later = harness.collect_for(Duration::from_millis(100));
    assert!(
        !later
            .iter()
            .any(|event| matches!(event, RunnerEvent::SettingsWidgets(_))),
        "{later:#?}"
    );
}

#[test]
fn reports_every_kind_of_widget_in_order_with_its_tooltip() {
    use livesplit_auto_splitting::settings::WidgetKind;

    let harness = Harness::new();
    let path = harness.write("widgets.wasm", WIDGETS);
    harness.load(&path);
    let events = harness.wait_for(|event| matches!(event, RunnerEvent::SettingsWidgets(_)));
    let Some(RunnerEvent::SettingsWidgets(widgets)) = events.last() else {
        unreachable!()
    };
    let kinds: Vec<_> = widgets
        .0
        .iter()
        .map(|widget| match &widget.kind {
            WidgetKind::Title { heading_level } => format!("{} title {heading_level}", widget.key),
            WidgetKind::Bool { default_value } => format!("{} bool {default_value}", widget.key),
            WidgetKind::Choice { options, .. } => {
                format!("{} choice {}", widget.key, options.len())
            }
            WidgetKind::FileSelect { filters } => format!("{} file {}", widget.key, filters.len()),
            WidgetKind::TextInput { default_value } => {
                format!("{} text {default_value}", widget.key)
            }
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "splits title 0",
            "gym bool true",
            "extras title 1",
            "category choice 1",
            "route file 1",
            "runner text me",
        ]
    );
    assert_eq!(
        widgets.0[1].tooltip.as_deref(),
        Some("Split when the mission is passed")
    );
}
