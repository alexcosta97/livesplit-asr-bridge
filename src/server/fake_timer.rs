//! A fake LiveSplit One for tests: it connects to the Server, keeps a small
//! timer, and answers the server protocol as LiveSplit One does, with an
//! event before the answer to a command that changes the timer.

use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{client_async, tungstenite::Message};

/// Long enough for a slow CI machine; only reached when a test fails.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The fake timer's own state.
#[derive(Debug, Clone, Default)]
pub struct Model {
    names: Vec<String>,
    /// `None` while not running.
    index: Option<usize>,
    paused: bool,
    /// Whether each segment before `index` was split, not skipped.
    splits: Vec<bool>,
}

impl Model {
    /// A timer with segments named `names`, not running.
    pub fn new(names: &[&str]) -> Self {
        Self {
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            ..Self::default()
        }
    }

    /// Carries out a command that changes the timer, returning its event,
    /// or its error code.
    pub fn act(&mut self, command: &str) -> Result<&'static str, &'static str> {
        let len = self.names.len();
        match (command, self.index) {
            ("start", None) => {
                self.index = Some(0);
                self.splits.clear();
                Ok("Started")
            }
            ("start", Some(_)) => Err("RunAlreadyInProgress"),
            ("reset", Some(_)) => {
                self.index = None;
                self.paused = false;
                self.splits.clear();
                Ok("Reset")
            }
            ("split" | "skipSplit" | "undoSplit" | "reset", None) => Err("NoRunInProgress"),
            ("resume", Some(_)) if self.paused => {
                self.paused = false;
                Ok("Resumed")
            }
            ("resume", Some(_)) => Err("NotPaused"),
            ("pause", Some(_)) if self.paused => Err("AlreadyPaused"),
            (_, Some(_)) if self.paused && command != "undoSplit" => Err("TimerPaused"),
            ("split", Some(index)) if index < len => {
                self.splits.push(true);
                self.index = Some(index + 1);
                Ok(if index + 1 == len {
                    "Finished"
                } else {
                    "Splitted"
                })
            }
            ("skipSplit", Some(index)) if index + 1 < len => {
                self.splits.push(false);
                self.index = Some(index + 1);
                Ok("SplitSkipped")
            }
            ("skipSplit", Some(_)) => Err("CantSkipLastSplit"),
            ("undoSplit", Some(index)) if index > 0 => {
                self.splits.pop();
                self.index = Some(index - 1);
                Ok("SplitUndone")
            }
            ("undoSplit", Some(_)) => Err("CantUndoFirstSplit"),
            ("pause", Some(_)) => {
                self.paused = true;
                Ok("Paused")
            }
            _ => Err("RunFinished"),
        }
    }

    fn state(&self) -> Value {
        match self.index {
            None => json!({ "state": "NotRunning" }),
            Some(index) if index == self.names.len() => json!({ "state": "Ended" }),
            Some(index) if self.paused => json!({ "state": "Paused", "index": index }),
            Some(index) => json!({ "state": "Running", "index": index }),
        }
    }

    /// The messages sent back for a command.
    fn handle(&mut self, text: &str) -> Vec<Value> {
        let command: Value = serde_json::from_str(text).unwrap();
        let name = command["command"].as_str().unwrap_or_default();
        let index = command["index"].as_u64().map(|index| index as usize);
        let answer = |value: Value| json!({ "success": value });
        let error = |code: &str| json!({ "error": { "code": code } });
        match name {
            "getCurrentState" => vec![answer(self.state())],
            "getSegmentName" => match index.and_then(|index| self.names.get(index)) {
                Some(name) => vec![answer(json!(name))],
                None => vec![error("InvalidIndex")],
            },
            "getCurrentRunSplitTime" => match index {
                Some(index) if index < self.names.len() => {
                    let split = self.splits.get(index).copied().unwrap_or(false);
                    vec![answer(if split { json!("1:23.45") } else { Value::Null })]
                }
                _ => vec![error("InvalidIndex")],
            },
            "setGameTime" => vec![json!({ "event": "GameTimeSet" }), answer(Value::Null)],
            "pauseGameTime" | "resumeGameTime" | "setCustomVariable" => {
                vec![answer(Value::Null)]
            }
            _ => match self.act(name) {
                Ok(event) => vec![json!({ "event": event }), answer(Value::Null)],
                Err(code) => vec![error(code)],
            },
        }
    }
}

/// What the test asks of the fake timer.
enum Control {
    /// Carry out a command on the timer itself, as a hotkey would, sending
    /// its event or not.
    Act {
        command: String,
        event: bool,
    },
    Disconnect,
}

/// A fake LiveSplit One connected to the Server.
pub struct FakeTimer {
    control: mpsc::UnboundedSender<Control>,
    received: Arc<Mutex<Vec<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeTimer {
    /// Connects to the Server on `port`, with the timer in `model`'s state.
    pub fn connect(port: u16, model: Model) -> Self {
        let (control, mut controls) = mpsc::unbounded_channel();
        let received = Arc::new(Mutex::new(Vec::new()));
        let (connected, is_connected) = std::sync::mpsc::channel();
        let thread = std::thread::spawn({
            let received = received.clone();
            move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async move {
                    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
                    let url = format!("ws://127.0.0.1:{port}/");
                    let (mut socket, _) = client_async(url, stream).await.unwrap();
                    let _ = connected.send(());
                    let mut model = model;
                    loop {
                        let replies = tokio::select! {
                            message = socket.next() => match message {
                                Some(Ok(Message::Text(text))) => {
                                    received.lock().unwrap().push(text.to_string());
                                    model.handle(&text)
                                }
                                Some(Ok(_)) => Vec::new(),
                                _ => return,
                            },
                            control = controls.recv() => match control {
                                Some(Control::Act { command, event }) => match model.act(&command) {
                                    Ok(name) if event => vec![json!({ "event": name })],
                                    _ => Vec::new(),
                                },
                                Some(Control::Disconnect) | None => {
                                    let _ = socket.close(None).await;
                                    return;
                                }
                            },
                        };
                        for reply in replies {
                            if socket.send(Message::text(reply.to_string())).await.is_err() {
                                return;
                            }
                        }
                    }
                });
            }
        });
        is_connected
            .recv_timeout(TIMEOUT)
            .expect("the fake timer didn't connect");
        Self {
            control,
            received,
            thread: Some(thread),
        }
    }

    /// Carries out `command` on the timer itself, as a hotkey would. With
    /// `event`, the timer reports it; without, the event is lost.
    pub fn act(&self, command: &str, event: bool) {
        let command = command.to_owned();
        let _ = self.control.send(Control::Act { command, event });
    }

    /// Every message the timer received, in order.
    pub fn received(&self) -> Vec<String> {
        self.received.lock().unwrap().clone()
    }

    /// The commands the timer received for the auto splitter's actions,
    /// leaving out the Server's own questions.
    pub fn actions(&self) -> Vec<String> {
        self.received()
            .into_iter()
            .filter(|text| !text.contains(r#""command":"get"#))
            .collect()
    }

    /// Closes the connection.
    pub fn disconnect(mut self) {
        let _ = self.control.send(Control::Disconnect);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for FakeTimer {
    fn drop(&mut self) {
        let _ = self.control.send(Control::Disconnect);
    }
}

/// Waits until `condition` holds, failing the test on timeout.
pub fn eventually(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + TIMEOUT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}
