//! The Runner: loads a `.wasm` auto splitter into the upstream
//! `livesplit-auto-splitting` runtime and runs it on a dedicated thread.
//!
//! The auto splitter controls a timer through [`TimerLink`], which the Server
//! implements. Everything that happens (loads, crashes, log messages, timer
//! actions, the game being attached) is reported as a [`RunnerEvent`].
//!
//! Each auto splitter starts with the settings it is given, which are the
//! saved settings of its game (spec §7.2).
//!
//! A hung auto splitter never blocks the caller: loading compiles on a
//! background thread, and reload, unload and dropping the [`Runner`]
//! interrupt the running auto splitter through the runtime.

mod events;
mod link;
mod thread;
mod timer;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{self, Receiver, Sender},
    },
    thread::JoinHandle,
};

use livesplit_auto_splitting::{AutoSplitter, Config, InterruptHandle, Runtime, settings};

pub use events::{RunnerEvent, Widgets, file_name};
pub use link::{NoTimer, TimerAction, TimerLink};

use events::EventSink;
use thread::Command;
use timer::BridgeTimer;

/// Runs one auto splitter at a time. Dropping it stops the auto splitter,
/// interrupting it if it hangs, and waits for its thread to finish.
pub struct Runner {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

struct Shared {
    control: Mutex<Control>,
    commands: Sender<Command>,
    events: EventSink,
    link: Arc<dyn TimerLink>,
}

#[derive(Default)]
struct Control {
    /// The file of the loaded auto splitter, which Reload loads again.
    path: Option<PathBuf>,
    /// Interrupts the loaded auto splitter. Each auto splitter has its own
    /// runtime, so interrupting one never affects another.
    interrupt: Option<InterruptHandle>,
    /// Counts load and unload requests, so a slow load that finishes after a
    /// newer request is discarded.
    latest_request: u64,
    shut_down: bool,
}

impl Runner {
    /// Starts the Runner thread, with no auto splitter loaded. Events arrive
    /// on the returned receiver, and `wake` is called after each one, for
    /// example to repaint the window.
    pub fn new(
        link: Arc<dyn TimerLink>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> (Self, Receiver<RunnerEvent>) {
        let (events, receiver) = EventSink::new(wake);
        let (commands, command_receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("auto splitter".into())
            .spawn({
                let events = events.clone();
                move || thread::run(command_receiver, events)
            })
            .expect("failed to start the auto splitter thread");

        let shared = Arc::new(Shared {
            control: Mutex::default(),
            commands,
            events,
            link,
        });
        let runner = Self {
            shared,
            thread: Some(thread),
        };
        (runner, receiver)
    }

    /// Loads the auto splitter at `path` in the background, starting it with
    /// `settings`. On success it replaces the running auto splitter; on
    /// failure the running one keeps running. Either way, the outcome is
    /// reported as an event.
    pub fn load(&self, path: PathBuf, settings: settings::Map) {
        let request = {
            let mut control = self.shared.control();
            control.latest_request += 1;
            control.latest_request
        };
        let shared = self.shared.clone();
        let spawned = std::thread::Builder::new()
            .name("auto splitter loader".into())
            .spawn({
                let path = path.clone();
                move || shared.finish_load(path, settings, request)
            });
        if let Err(error) = spawned {
            self.shared.events.send(RunnerEvent::LoadFailed {
                path,
                error: format!("couldn't start loading: {error}"),
            });
        }
    }

    /// Loads the same file again with `settings`, for example after the auto
    /// splitter was rebuilt. Does nothing if no auto splitter has been
    /// loaded.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the UI reloads through `load`, with the game's settings"
        )
    )]
    pub fn reload(&self, settings: settings::Map) {
        let path = self.shared.control().path.clone();
        if let Some(path) = path {
            self.load(path, settings);
        }
    }

    /// Gives the running auto splitter new settings, between two ticks. An
    /// auto splitter still loading keeps the settings it was loaded with.
    pub fn set_settings(&self, settings: settings::Map) {
        let _ = self.shared.commands.send(Command::SetSettings(settings));
    }

    /// Stops and unloads the running auto splitter, interrupting it if it
    /// hangs. Loads still in progress are discarded.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the UI has no unload action yet")
    )]
    pub fn unload(&self) {
        let mut control = self.shared.control();
        control.latest_request += 1;
        control.path = None;
        let _ = self.shared.commands.send(Command::Unload);
        if let Some(interrupt) = control.interrupt.take() {
            interrupt.interrupt();
        }
        self.shared.events.send(RunnerEvent::Unloaded);
    }

    /// The file of the loaded auto splitter, if any.
    pub fn loaded_path(&self) -> Option<PathBuf> {
        self.shared.control().path.clone()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        {
            let mut control = self.shared.control();
            control.shut_down = true;
            let _ = self.shared.commands.send(Command::Shutdown);
            if let Some(interrupt) = control.interrupt.take() {
                interrupt.interrupt();
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Shared {
    fn control(&self) -> MutexGuard<'_, Control> {
        // Nothing panics while holding the lock, but recover anyway rather
        // than take the app down.
        self.control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Runs on a loader thread: compiles the auto splitter, then hands it to
    /// the Runner thread unless a newer request arrived in the meantime.
    fn finish_load(&self, path: PathBuf, settings: settings::Map, request: u64) {
        let result = self.instantiate(&path, settings);

        let mut control = self.control();
        if control.shut_down || control.latest_request != request {
            return;
        }
        match result {
            Err(error) => self.events.send(RunnerEvent::LoadFailed { path, error }),
            Ok((auto_splitter, interrupt)) => {
                // Reported before the auto splitter can tick, so its first
                // events come after this one.
                self.events.send(RunnerEvent::Loaded {
                    path: path.clone(),
                    tick_rate: auto_splitter.tick_rate(),
                });
                if self
                    .commands
                    .send(Command::Replace(Box::new(auto_splitter)))
                    .is_err()
                {
                    return;
                }
                // Sent first, so the Runner thread finds the replacement
                // when the interrupt stops the old auto splitter mid-tick.
                if let Some(old) = control.interrupt.replace(interrupt) {
                    old.interrupt();
                }
                control.path = Some(path);
            }
        }
    }

    fn instantiate(
        &self,
        path: &Path,
        settings: settings::Map,
    ) -> Result<(AutoSplitter<BridgeTimer>, InterruptHandle), String> {
        let module = fs::read(path).map_err(|error| format!("couldn't read the file: {error}"))?;
        let runtime = Runtime::new(Config::default()).map_err(|error| error_chain(&error))?;
        let timer = BridgeTimer::new(self.link.clone(), self.events.clone());
        let auto_splitter = runtime
            .compile(&module)
            .and_then(|compiled| compiled.instantiate(timer, Some(settings), None))
            .map_err(|error| error_chain(&error))?;
        let interrupt = auto_splitter.interrupt_handle();
        Ok((auto_splitter, interrupt))
    }
}

/// An error and its sources, as one line.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[cfg(test)]
mod tests;
