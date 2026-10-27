//! Searching speedrun.com for games in the background (spec §7.5).

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::{SpeedrunGame, parse_games};
use crate::version::VERSION;

/// The address of speedrun.com's API.
#[cfg_attr(test, allow(dead_code))] // Tests use a local server instead.
pub const API_BASE: &str = "https://www.speedrun.com/api/v1";

/// How many games one search asks for.
const MAX_GAMES: &str = "20";

/// How long a search may take before it counts as failed.
const TIMEOUT: Duration = Duration::from_secs(5);

/// The answer to one search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAnswer {
    /// The name that was searched for.
    pub name: String,
    /// The games found, or why the search failed.
    pub games: Result<Vec<SpeedrunGame>, String>,
}

/// Something that looks games up by name without blocking the caller.
pub trait GameSearch: Send {
    /// Starts looking for games named `name`. A name still waiting for its
    /// turn is dropped in favour of this one.
    fn request(&mut self, name: &str);

    /// The next finished search, if there is one.
    fn poll(&mut self) -> Option<SearchAnswer>;
}

/// What the caller and the worker thread share.
#[derive(Default)]
struct State {
    /// The name waiting for the worker, if any.
    waiting: Option<String>,
    /// Whether the search was dropped and the worker should stop.
    closed: bool,
}

/// The state and the signal that wakes the worker when it changes.
type Shared = Arc<(Mutex<State>, Condvar)>;

/// Searches speedrun.com on a worker thread, one name at a time.
pub struct SpeedrunSearch {
    /// The state shared with the worker.
    shared: Shared,
    /// The answers coming back from the worker.
    answers: Receiver<SearchAnswer>,
}

impl SpeedrunSearch {
    /// Starts a worker that searches the API at `base_url`.
    pub fn start(base_url: &str) -> Self {
        let shared: Shared = Arc::default();
        let (sender, answers) = mpsc::channel();
        let worker_shared = shared.clone();
        let base_url = base_url.to_string();
        std::thread::spawn(move || work(&base_url, &worker_shared, &sender));
        Self { shared, answers }
    }
}

impl GameSearch for SpeedrunSearch {
    fn request(&mut self, name: &str) {
        let (state, wake) = &*self.shared;
        state.lock().unwrap().waiting = Some(name.to_string());
        wake.notify_one();
    }

    fn poll(&mut self) -> Option<SearchAnswer> {
        self.answers.try_recv().ok()
    }
}

impl Drop for SpeedrunSearch {
    /// Tells the worker to stop without waiting for a search in progress.
    fn drop(&mut self) {
        let (state, wake) = &*self.shared;
        state.lock().unwrap().closed = true;
        wake.notify_one();
    }
}

/// The worker: takes the waiting name, searches for it and sends the answer,
/// until the search is dropped or nobody listens any more.
fn work(base_url: &str, shared: &Shared, sender: &Sender<SearchAnswer>) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(format!("livesplit-asr-bridge/{VERSION}"))
        .build()
        .into();
    let (state, wake) = &**shared;
    loop {
        let name = {
            let mut guard = state.lock().unwrap();
            loop {
                if guard.closed {
                    return;
                }
                if let Some(name) = guard.waiting.take() {
                    break name;
                }
                guard = wake.wait(guard).unwrap();
            }
        };
        let games = fetch(&agent, base_url, &name);
        if sender.send(SearchAnswer { name, games }).is_err() {
            return;
        }
    }
}

/// Asks the API at `base_url` for the games named `name`.
fn fetch(agent: &ureq::Agent, base_url: &str, name: &str) -> Result<Vec<SpeedrunGame>, String> {
    let mut response = agent
        .get(format!("{base_url}/games"))
        .query("name", name)
        .query("max", MAX_GAMES)
        .call()
        .map_err(|e| e.to_string())?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_games(&body)
}

/// Builds the search the app uses: the real speedrun.com.
#[cfg_attr(test, allow(dead_code))] // Tests of the app use `Offline` instead.
pub fn live() -> Box<dyn GameSearch> {
    Box::new(SpeedrunSearch::start(API_BASE))
}

/// A search that never answers, for tests of the app that must not use the
/// network.
#[cfg(test)]
pub struct Offline;

#[cfg(test)]
impl GameSearch for Offline {
    fn request(&mut self, _name: &str) {}

    fn poll(&mut self) -> Option<SearchAnswer> {
        None
    }
}

/// What a [`FakeSearch`] and its clones share.
#[cfg(test)]
#[derive(Default)]
struct FakeState {
    /// The names asked for, in order.
    requests: Vec<String>,
    /// The answers to hand out, oldest first.
    answers: std::collections::VecDeque<SearchAnswer>,
}

/// A search for tests: answers only what the test queued. Clones share their
/// state, so a test can box one clone and keep another to queue answers and
/// read requests.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct FakeSearch {
    /// The state shared by all clones.
    state: Arc<Mutex<FakeState>>,
}

#[cfg(test)]
impl FakeSearch {
    /// Queues an answer for `poll` to hand out.
    pub fn answer(&self, name: &str, games: Result<Vec<SpeedrunGame>, String>) {
        self.state.lock().unwrap().answers.push_back(SearchAnswer {
            name: name.to_string(),
            games,
        });
    }

    /// The names asked for so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.state.lock().unwrap().requests.clone()
    }
}

#[cfg(test)]
impl GameSearch for FakeSearch {
    fn request(&mut self, name: &str) {
        self.state.lock().unwrap().requests.push(name.to_string());
    }

    fn poll(&mut self) -> Option<SearchAnswer> {
        self.state.lock().unwrap().answers.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::version::VERSION;

    const GAME_JSON: &str = r#"{"data":[{"id":"abc","names":{"international":"Grand Theft Auto: San Andreas"},"abbreviation":"gtasa","released":2004}]}"#;

    /// What a test server received for one request.
    struct Received {
        request_line: String,
        headers: Vec<String>,
    }

    /// Serves HTTP on 127.0.0.1, one thread per connection. `respond` gets the
    /// request and the number of the connection (from 0) and returns the whole
    /// response, or `None` to leave the connection unanswered. Every request is
    /// also sent on the returned channel.
    fn serve<F>(respond: F) -> (SocketAddr, mpsc::Receiver<Received>)
    where
        F: Fn(&Received, usize) -> Option<String> + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (seen_tx, seen_rx) = mpsc::channel();
        let respond = Arc::new(respond);
        std::thread::spawn(move || {
            for (number, stream) in listener.incoming().enumerate() {
                let Ok(mut stream) = stream else { continue };
                let seen_tx = seen_tx.clone();
                let respond = respond.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
                        return;
                    }
                    let mut headers = Vec::new();
                    loop {
                        let mut line = String::new();
                        if reader.read_line(&mut line).unwrap_or(0) == 0 {
                            return;
                        }
                        let line = line.trim_end().to_string();
                        if line.is_empty() {
                            break;
                        }
                        headers.push(line);
                    }
                    let received = Received {
                        request_line: request_line.trim_end().to_string(),
                        headers,
                    };
                    let response = {
                        // Report the request before answering, so a test can wait for it.
                        let _ = seen_tx.send(Received {
                            request_line: received.request_line.clone(),
                            headers: received.headers.clone(),
                        });
                        respond(&received, number)
                    };
                    if let Some(response) = response {
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.flush();
                    } else {
                        // Keep the connection open without answering.
                        std::thread::sleep(Duration::from_secs(10));
                    }
                });
            }
        });
        (addr, seen_rx)
    }

    /// A 200 response carrying `body`.
    fn ok(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    /// Polls `search` until it answers, for at most five seconds.
    fn wait_for_answer(search: &mut SpeedrunSearch) -> SearchAnswer {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(answer) = search.poll() {
                return answer;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("no answer within five seconds");
    }

    #[test]
    fn asks_for_twenty_games_by_name_with_the_app_s_user_agent() {
        let (addr, seen) = serve(|_, _| Some(ok(GAME_JSON)));
        let mut search = SpeedrunSearch::start(&format!("http://{addr}"));
        search.request("gta san andreas");
        let answer = wait_for_answer(&mut search);
        let received = seen.recv_timeout(Duration::from_secs(5)).unwrap();
        let target = received.request_line.split(' ').nth(1).unwrap();
        let decoded = target.replace("%20", " ").replace('+', " ");
        assert!(received.request_line.starts_with("GET /games?"), "{target}");
        assert!(decoded.contains("name=gta san andreas"), "{target}");
        assert!(decoded.contains("max=20"), "{target}");
        let user_agent = format!("user-agent: livesplit-asr-bridge/{VERSION}");
        assert!(
            received
                .headers
                .iter()
                .any(|h| h.eq_ignore_ascii_case(&user_agent)),
            "{:?}",
            received.headers
        );
        assert_eq!(answer.name, "gta san andreas");
        assert_eq!(
            answer.games,
            Ok(vec![SpeedrunGame {
                id: "abc".into(),
                name: "Grand Theft Auto: San Andreas".into(),
                abbreviation: "gtasa".into(),
                year: Some(2004),
            }])
        );
    }

    #[test]
    fn an_error_status_is_a_failed_answer() {
        let (addr, _seen) = serve(|_, _| {
            Some("HTTP/1.1 420 \r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into())
        });
        let mut search = SpeedrunSearch::start(&format!("http://{addr}"));
        search.request("portal");
        let answer = wait_for_answer(&mut search);
        assert_eq!(answer.name, "portal");
        assert!(answer.games.is_err());
    }

    #[test]
    fn a_newer_name_replaces_one_still_waiting() {
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let release_rx = Mutex::new(release_rx);
        let (addr, seen) = serve(move |_, number| {
            if number == 0 {
                let _ = release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5));
            }
            Some(ok(r#"{"data":[]}"#))
        });
        let mut search = SpeedrunSearch::start(&format!("http://{addr}"));
        search.request("a");
        // Wait until the worker is busy with "a" before asking for more.
        let _first = seen.recv_timeout(Duration::from_secs(5)).unwrap();
        search.request("b");
        search.request("c");
        release_tx.send(()).unwrap();
        let first = wait_for_answer(&mut search);
        let second = wait_for_answer(&mut search);
        assert_eq!([first.name, second.name], ["a", "c"]);
    }

    #[test]
    fn dropping_the_search_does_not_wait_for_the_worker() {
        let (addr, seen) = serve(|_, _| None);
        let mut search = SpeedrunSearch::start(&format!("http://{addr}"));
        search.request("a");
        seen.recv_timeout(Duration::from_secs(5)).unwrap();
        let started = Instant::now();
        drop(search);
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn a_fake_search_answers_what_it_was_given_and_shares_its_state_between_clones() {
        let fake = FakeSearch::default();
        let mut boxed: Box<dyn GameSearch> = Box::new(fake.clone());
        fake.answer("portal", Ok(Vec::new()));
        boxed.request("portal");
        assert_eq!(fake.requests(), vec!["portal".to_string()]);
        let answer = boxed.poll().unwrap();
        assert_eq!(answer.name, "portal");
        assert!(boxed.poll().is_none());
    }

    #[test]
    fn the_offline_search_never_answers() {
        let mut offline = Offline;
        offline.request("portal");
        assert!(offline.poll().is_none());
    }
}
