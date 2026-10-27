//! A speedrun.com game's category variants, read from the answer to the
//! request for its categories and levels (spec §7.5.1).

use std::collections::BTreeMap;
use std::fmt;
use std::sync::mpsc::{self, Receiver};

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::search::agent;

/// The most variants one category yields; the rest are left out.
const MAX_VARIANTS_PER_CATEGORY: usize = 1_000;
/// The most variants the whole answer yields; the rest are left out.
const MAX_VARIANTS: usize = 10_000;

/// The ids that link a category here to a variant on speedrun.com.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CategoryLink {
    /// speedrun.com's id for the category.
    pub id: String,
    /// The id of the chosen value of each subcategory, by variable id.
    pub values: BTreeMap<String, String>,
    /// The id of the level, for a level variant.
    pub level: Option<String>,
}

/// One way to run a category on speedrun.com: the category with one value of
/// each of its subcategories, for the whole game or for a level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The name LiveSplit gives it, like "Any% (PC, Glitchless)".
    pub name: String,
    /// The ids that identify it on speedrun.com.
    pub link: CategoryLink,
}

impl Variant {
    /// Whether this is a variant of a level rather than of the whole game.
    pub fn is_level(&self) -> bool {
        self.link.level.is_some()
    }
}

/// The answer to one request: the game asked for, and its variants or why
/// they could not be read.
type Answer = (String, Result<Vec<Variant>, String>);

/// Something that fetches a game's variants without blocking the caller.
pub trait CategorySource: Send {
    /// Starts fetching the variants of the speedrun.com game `game_id`.
    fn request(&mut self, game_id: &str);

    /// The answer, once: the game id asked for and its variants or error.
    fn poll(&mut self) -> Option<Answer>;
}

/// Fetches variants from speedrun.com, each request on a thread of its own.
pub struct SpeedrunCategories {
    /// The address of the API.
    base_url: String,
    /// Where the threads send their answers.
    sender: mpsc::Sender<Answer>,
    /// The answers coming back from the threads.
    answers: Receiver<Answer>,
}

impl SpeedrunCategories {
    /// A source that asks the API at `base_url`.
    pub fn new(base_url: &str) -> Self {
        let (sender, answers) = mpsc::channel();
        Self {
            base_url: base_url.to_string(),
            sender,
            answers,
        }
    }
}

impl CategorySource for SpeedrunCategories {
    fn request(&mut self, game_id: &str) {
        if !valid_id(game_id) {
            let _ = self.sender.send((
                game_id.to_string(),
                Err(format!("{game_id:?} is not a game id")),
            ));
            return;
        }
        let base_url = self.base_url.clone();
        let sender = self.sender.clone();
        let game_id = game_id.to_string();
        std::thread::spawn(move || {
            let variants = fetch(&agent(), &base_url, &game_id);
            // Nobody listens once the source is dropped; that is fine.
            let _ = sender.send((game_id, variants));
        });
    }

    fn poll(&mut self) -> Option<Answer> {
        self.answers.try_recv().ok()
    }
}

/// Whether `id` can be put in an address: not empty, and only ASCII letters,
/// digits, `_` and `-`. A hand-edited file may hold anything as a game id.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Asks the API at `base_url` for the game `game_id` with its levels,
/// categories and their variables.
fn fetch(agent: &ureq::Agent, base_url: &str, game_id: &str) -> Result<Vec<Variant>, String> {
    let mut response = agent
        .get(format!("{base_url}/games/{game_id}"))
        .query("embed", "levels.categories.variables,categories.variables")
        .call()
        .map_err(|e| e.to_string())?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_variants(&body)
}

/// Builds the source the app uses: the real speedrun.com.
#[cfg(not(test))]
pub fn live_categories() -> Box<dyn CategorySource> {
    Box::new(SpeedrunCategories::new(super::search::API_BASE))
}

/// A source that never answers, for tests of the app that must not use the
/// network.
#[cfg(test)]
pub struct OfflineCategories;

#[cfg(test)]
impl CategorySource for OfflineCategories {
    fn request(&mut self, _game_id: &str) {}

    fn poll(&mut self) -> Option<Answer> {
        None
    }
}

/// What a [`FakeCategories`] and its clones share.
#[cfg(test)]
#[derive(Default)]
struct FakeState {
    /// The game ids asked for, in order.
    requests: Vec<String>,
    /// The answers to hand out, oldest first.
    answers: std::collections::VecDeque<Answer>,
}

/// A source for tests: answers only what the test queued. Clones share their
/// state, so a test can box one clone and keep another to queue answers and
/// read requests.
#[cfg(test)]
#[derive(Clone, Default)]
pub struct FakeCategories {
    /// The state shared by all clones.
    state: std::sync::Arc<std::sync::Mutex<FakeState>>,
}

#[cfg(test)]
impl FakeCategories {
    /// Queues an answer for `poll` to hand out.
    pub fn answer(&self, game_id: &str, variants: Result<Vec<Variant>, String>) {
        let mut state = self.state.lock().unwrap();
        state.answers.push_back((game_id.to_string(), variants));
    }

    /// The game ids asked for so far, in order.
    pub fn requests(&self) -> Vec<String> {
        self.state.lock().unwrap().requests.clone()
    }
}

#[cfg(test)]
impl CategorySource for FakeCategories {
    fn request(&mut self, game_id: &str) {
        self.state
            .lock()
            .unwrap()
            .requests
            .push(game_id.to_string());
    }

    fn poll(&mut self) -> Option<Answer> {
        self.state.lock().unwrap().answers.pop_front()
    }
}

/// The body of the answer, as speedrun.com sends it.
#[derive(Deserialize)]
struct Response {
    data: RawGame,
}

/// The game of the answer, with its levels and categories embedded.
#[derive(Deserialize)]
struct RawGame {
    levels: Option<Embedded<RawLevel>>,
    categories: Option<Embedded<RawCategory>>,
}

/// An embedded list, which speedrun.com wraps in an object.
#[derive(Deserialize)]
struct Embedded<T> {
    data: Vec<T>,
}

/// A level and the categories listed under it.
#[derive(Deserialize)]
struct RawLevel {
    id: String,
    name: String,
    categories: Option<Embedded<RawCategory>>,
}

/// A category and its variables.
#[derive(Deserialize)]
struct RawCategory {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    variables: Option<Embedded<RawVariable>>,
}

/// A variable of a category.
#[derive(Deserialize)]
struct RawVariable {
    id: String,
    #[serde(rename = "is-subcategory", default)]
    is_subcategory: bool,
    scope: Option<RawScope>,
    #[serde(default)]
    values: RawValues,
}

/// Where a variable applies.
#[derive(Deserialize)]
struct RawScope {
    #[serde(rename = "type")]
    kind: String,
    level: Option<String>,
}

/// The values a variable can take.
#[derive(Deserialize, Default)]
struct RawValues {
    #[serde(default)]
    values: Labels,
}

/// The values of a variable as (value id, label), in the order of the
/// answer, which a map would lose.
#[derive(Default)]
struct Labels(Vec<(String, String)>);

impl<'de> Deserialize<'de> for Labels {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// The label of a value.
        #[derive(Deserialize)]
        struct Label {
            label: String,
        }

        struct LabelsVisitor;

        impl<'de> Visitor<'de> for LabelsVisitor {
            type Value = Labels;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map of values")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Labels, A::Error> {
                let mut values = Vec::new();
                while let Some((id, Label { label })) = map.next_entry::<String, Label>()? {
                    values.push((id, label));
                }
                Ok(Labels(values))
            }
        }

        deserializer.deserialize_map(LabelsVisitor)
    }
}

/// Reads the variants out of the answer: those of the whole game first, then
/// those of each level, all in the order of the answer.
pub fn parse_variants(json: &str) -> Result<Vec<Variant>, String> {
    let response: Response = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let game = response.data;
    let mut variants = Vec::new();
    let full_game = game.categories.into_iter().flat_map(|c| c.data);
    for category in full_game.filter(|c| c.kind == "per-game") {
        push_variants(&mut variants, &category, None, "");
    }
    for level in game.levels.into_iter().flat_map(|l| l.data) {
        let prefix = format!("{}: ", level.name);
        let categories = level.categories.into_iter().flat_map(|c| c.data);
        for category in categories.filter(|c| c.kind == "per-level") {
            push_variants(&mut variants, &category, Some(&level.id), &prefix);
        }
    }
    variants.truncate(MAX_VARIANTS);
    Ok(variants)
}

/// Adds a variant for each combination of one value of each subcategory of
/// the category that applies to the whole game (`level` is `None`) or to that
/// level. The first subcategory changes slowest. Stops at the limits on
/// variants, without building the combinations that would pass them.
fn push_variants(
    variants: &mut Vec<Variant>,
    category: &RawCategory,
    level: Option<&str>,
    prefix: &str,
) {
    let room = MAX_VARIANTS.saturating_sub(variants.len());
    if room == 0 {
        return;
    }
    // No combination past this many is ever used.
    let cap = room.min(MAX_VARIANTS_PER_CATEGORY);
    // Each combination so far: the chosen value of each variable, and labels.
    let mut combinations: Vec<(BTreeMap<String, String>, Vec<&str>)> =
        vec![(BTreeMap::new(), Vec::new())];
    let variables = category.variables.iter().flat_map(|v| &v.data);
    for variable in variables.filter(|v| v.is_subcategory && applies(v.scope.as_ref(), level)) {
        if variable.values.values.0.is_empty() {
            continue;
        }
        combinations = combinations
            .iter()
            .take(cap)
            .flat_map(|(chosen, labels)| {
                variable.values.values.0.iter().map(move |(id, label)| {
                    let mut chosen = chosen.clone();
                    chosen.insert(variable.id.clone(), id.clone());
                    let mut labels = labels.clone();
                    labels.push(label);
                    (chosen, labels)
                })
            })
            .take(cap)
            .collect();
    }
    for (values, labels) in combinations {
        let name = if labels.is_empty() {
            format!("{prefix}{}", category.name)
        } else {
            format!("{prefix}{} ({})", category.name, labels.join(", "))
        };
        variants.push(Variant {
            name,
            link: CategoryLink {
                id: category.id.clone(),
                values,
                level: level.map(str::to_owned),
            },
        });
    }
}

/// Whether a variable with this scope applies to the whole game (`level` is
/// `None`) or to that level.
fn applies(scope: Option<&RawScope>, level: Option<&str>) -> bool {
    let Some(scope) = scope else {
        return false;
    };
    match (scope.kind.as_str(), level) {
        ("global", _) => true,
        ("full-game", None) => true,
        ("all-levels", Some(_)) => true,
        ("single-level", Some(id)) => scope.level.as_deref() == Some(id),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::speedrun::test_server::{ok, serve};
    use crate::version::VERSION;

    /// A variable of a category, as speedrun.com sends it.
    fn variable(id: &str, subcategory: bool, scope: &str, values: &[(&str, &str)]) -> String {
        let values: Vec<String> = values
            .iter()
            .map(|(id, label)| format!(r#""{id}":{{"label":"{label}","rules":""}}"#))
            .collect();
        format!(
            r#"{{"id":"{id}","name":"{id}","is-subcategory":{subcategory},"scope":{scope},
              "values":{{"values":{{{}}},"default":"x"}}}}"#,
            values.join(",")
        )
    }

    /// A category of the given type, with variables.
    fn category(id: &str, name: &str, kind: &str, variables: &[String]) -> String {
        format!(
            r#"{{"id":"{id}","name":"{name}","type":"{kind}","miscellaneous":false,
              "variables":{{"data":[{}]}}}}"#,
            variables.join(",")
        )
    }

    fn answer(levels: &str, categories: &[String]) -> String {
        format!(
            r#"{{"data":{{"id":"g","levels":{{"data":[{levels}]}},
              "categories":{{"data":[{}]}}}}}}"#,
            categories.join(",")
        )
    }

    const GLOBAL: &str = r#"{"type":"global"}"#;

    fn platform() -> String {
        variable("platform", true, GLOBAL, &[("pc", "PC"), ("n64", "N64")])
    }

    /// The game of the examples: two full-game categories and a level one,
    /// and two levels with the level category.
    fn mario() -> String {
        let verified = variable("verified", false, GLOBAL, &[("yes", "Yes")]);
        let codes = variable("codes", true, r#"{"type":"all-levels"}"#, &[("on", "On")]);
        let star = variable(
            "star",
            true,
            r#"{"type":"single-level","level":"whomp"}"#,
            &[("1", "1"), ("2", "2")],
        );
        let stage = category(
            "stage",
            "Stage RTA",
            "per-level",
            &[platform(), codes, star],
        );
        let stray = category("stray", "Stray", "per-game", &[]);
        let levels = format!(
            r#"{{"id":"bob","name":"Bob-omb Battlefield","categories":{{"data":[{stage},{stray}]}}}},
              {{"id":"whomp","name":"Whomp's Fortress","categories":{{"data":[{stage}]}}}}"#
        );
        answer(
            &levels,
            &[
                category("any", "Any%", "per-game", &[platform(), verified]),
                category("zero", "0 Star", "per-game", &[]),
                category("stage", "Stage RTA", "per-level", &[]),
            ],
        )
    }

    fn names(variants: &[Variant]) -> Vec<&str> {
        variants.iter().map(|v| v.name.as_str()).collect()
    }

    #[test]
    fn reads_full_game_variants_in_order() {
        let variants = parse_variants(&mario()).unwrap();
        assert_eq!(names(&variants)[..3], ["Any% (PC)", "Any% (N64)", "0 Star"]);
        assert_eq!(
            variants[0].link,
            CategoryLink {
                id: "any".into(),
                values: BTreeMap::from([("platform".into(), "pc".into())]),
                level: None,
            }
        );
        assert!(variants[2].link.values.is_empty());
        assert!(variants[..3].iter().all(|v| !v.is_level()));
        assert!(variants[3..].iter().all(Variant::is_level));
        assert!(!names(&variants[..3]).contains(&"Stage RTA"));
    }

    #[test]
    fn level_variants_take_only_their_level_s_subcategories() {
        let variants = parse_variants(&mario()).unwrap();
        let bob = "Bob-omb Battlefield: Stage RTA (PC, On)";
        let whomp = "Whomp's Fortress: Stage RTA (PC, On, 1)";
        assert!(names(&variants).contains(&bob));
        assert!(!variants.iter().any(|v| v.name.contains("Stray")));
        assert!(
            variants
                .iter()
                .filter(|v| v.name.starts_with("Bob-omb"))
                .all(|v| !v.link.values.contains_key("star"))
        );
        let whomp = variants.iter().find(|v| v.name == whomp).unwrap();
        assert_eq!(whomp.link.level.as_deref(), Some("whomp"));
        assert_eq!(whomp.link.values["star"], "1");
    }

    #[test]
    fn values_keep_the_order_of_the_answer() {
        let order = variable("v", true, GLOBAL, &[("zz", "Zed"), ("aa", "Ay")]);
        let json = answer("", &[category("c", "Run", "per-game", &[order])]);
        assert_eq!(
            names(&parse_variants(&json).unwrap()),
            ["Run (Zed)", "Run (Ay)"]
        );
    }

    #[test]
    fn missing_parts_read_as_empty() {
        assert!(parse_variants(r#"{"data":{"id":"g"}}"#).unwrap().is_empty());
        let json = r#"{"data":{"levels":{"data":[{"id":"l","name":"L"}]},
          "categories":{"data":[{"id":"c","name":"Run","type":"per-game"}]}}}"#;
        assert_eq!(names(&parse_variants(json).unwrap()), ["Run"]);
        let empty = variable("v", true, GLOBAL, &[]);
        let json = answer("", &[category("c", "Run", "per-game", &[empty])]);
        assert_eq!(names(&parse_variants(&json).unwrap()), ["Run"]);
        let bare = r#"{"id":"v"}"#.to_string();
        let json = answer("", &[category("c", "Run", "per-game", &[bare])]);
        assert_eq!(names(&parse_variants(&json).unwrap()), ["Run"]);
        let no_scope =
            r#"{"id":"v","is-subcategory":true,"values":{"values":{"a":{"label":"A"}}}}"#;
        let json = answer("", &[category("c", "Run", "per-game", &[no_scope.into()])]);
        assert_eq!(names(&parse_variants(&json).unwrap()), ["Run"]);
    }

    #[test]
    fn variants_stop_at_the_caps() {
        let labels: Vec<(String, String)> = (0..20)
            .map(|i| (format!("v{i}"), format!("L{i}")))
            .collect();
        let labels: Vec<(&str, &str)> = labels
            .iter()
            .map(|(a, b)| (a.as_str(), b.as_str()))
            .collect();
        let vars: Vec<String> = ["a", "b", "c"]
            .iter()
            .map(|id| variable(id, true, GLOBAL, &labels))
            .collect();
        let big = category("big", "Big", "per-game", &vars);
        let variants = parse_variants(&answer("", std::slice::from_ref(&big))).unwrap();
        assert_eq!(variants.len(), 1_000);
        assert_eq!(variants[0].name, "Big (L0, L0, L0)");
        assert_eq!(variants[1].name, "Big (L0, L0, L1)");
        assert_eq!(variants[999].name, "Big (L2, L9, L19)");
        let many = vec![big; 11];
        assert_eq!(parse_variants(&answer("", &many)).unwrap().len(), 10_000);
    }

    #[test]
    fn only_letters_digits_underscores_and_hyphens_make_a_valid_id() {
        for id in ["yo1yv1q5", "a_b-C9"] {
            assert!(valid_id(id), "{id}");
        }
        for id in ["", "a/b", "a b", "a?x=1", "..", "é", "a#"] {
            assert!(!valid_id(id), "{id:?}");
        }
    }

    #[test]
    fn the_source_answers_an_error_for_an_invalid_id_without_asking() {
        // Nothing listens here; a request would answer a connection error
        // later, not at once.
        let mut source = SpeedrunCategories::new("http://127.0.0.1:1");
        source.request("../users");
        let (id, answer) = source.poll().expect("answers at once");
        assert_eq!(id, "../users");
        assert!(answer.unwrap_err().contains("not a game id"));
    }

    #[test]
    fn an_answer_without_data_is_an_error() {
        assert!(parse_variants(r#"{"status":420}"#).is_err());
        assert!(parse_variants("not json").is_err());
    }

    const GAME_JSON: &str = r#"{"data":{"categories":{"data":[
      {"id":"cat","name":"Any%","type":"per-game","variables":{"data":[]}}]},
      "levels":{"data":[]}}}"#;

    /// Polls `source` until it answers, for at most five seconds.
    fn wait_for_answer(source: &mut SpeedrunCategories) -> Answer {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(answer) = source.poll() {
                return answer;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("no answer within five seconds");
    }

    #[test]
    fn asks_for_the_game_with_levels_and_categories_embedded() {
        let (addr, seen) = serve(|_, _| Some(ok(GAME_JSON)));
        let mut source = SpeedrunCategories::new(&format!("http://{addr}"));
        source.request("abc");
        let (game_id, variants) = wait_for_answer(&mut source);
        let received = seen.recv_timeout(Duration::from_secs(5)).unwrap();
        let target = received.request_line.split(' ').nth(1).unwrap();
        let decoded = target
            .replace("%2C", ",")
            .replace("%2c", ",")
            .replace("%2E", ".");
        assert_eq!(
            decoded,
            "/games/abc?embed=levels.categories.variables,categories.variables"
        );
        let user_agent = format!("user-agent: livesplit-asr-bridge/{VERSION}");
        assert!(
            received
                .headers
                .iter()
                .any(|h| h.eq_ignore_ascii_case(&user_agent)),
            "{:?}",
            received.headers
        );
        assert_eq!(game_id, "abc");
        let names: Vec<String> = variants.unwrap().into_iter().map(|v| v.name).collect();
        assert_eq!(names, ["Any%"]);
    }

    #[test]
    fn an_error_status_is_a_failed_answer() {
        let (addr, _seen) = serve(|_, _| {
            Some("HTTP/1.1 420 \r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into())
        });
        let mut source = SpeedrunCategories::new(&format!("http://{addr}"));
        source.request("abc");
        let (game_id, variants) = wait_for_answer(&mut source);
        assert_eq!(game_id, "abc");
        assert!(variants.is_err());
    }

    #[test]
    fn dropping_the_source_does_not_wait_for_the_worker() {
        let (addr, seen) = serve(|_, _| None);
        let mut source = SpeedrunCategories::new(&format!("http://{addr}"));
        source.request("abc");
        seen.recv_timeout(Duration::from_secs(5)).unwrap();
        let started = Instant::now();
        drop(source);
        assert!(started.elapsed() < Duration::from_millis(100));
    }

    #[test]
    fn a_fake_source_answers_what_it_was_given_and_shares_its_state_between_clones() {
        let fake = FakeCategories::default();
        let mut boxed: Box<dyn CategorySource> = Box::new(fake.clone());
        fake.answer("abc", Ok(Vec::new()));
        boxed.request("abc");
        assert_eq!(fake.requests(), vec!["abc".to_string()]);
        assert_eq!(boxed.poll(), Some(("abc".to_string(), Ok(Vec::new()))));
        assert!(boxed.poll().is_none());
    }

    #[test]
    fn the_offline_source_never_answers() {
        let mut offline = OfflineCategories;
        offline.request("abc");
        assert!(offline.poll().is_none());
    }
}
