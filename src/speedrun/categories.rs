//! A speedrun.com game's category variants, read from the answer to the
//! request for its categories and levels (spec §7.5.1).

#![cfg_attr(not(test), allow(dead_code))] // Used by the Category dialog, which comes later.

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

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
    #[serde(rename = "is-subcategory")]
    is_subcategory: bool,
    scope: RawScope,
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
#[derive(Deserialize)]
struct RawValues {
    values: Labels,
}

/// The values of a variable as (value id, label), in the order of the
/// answer, which a map would lose.
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
    for category in game.categories.into_iter().flat_map(|c| c.data) {
        if category.kind == "per-game" {
            push_variants(&mut variants, &category, None, "");
        }
    }
    for level in game.levels.into_iter().flat_map(|l| l.data) {
        let prefix = format!("{}: ", level.name);
        for category in level.categories.into_iter().flat_map(|c| c.data) {
            push_variants(&mut variants, &category, Some(&level.id), &prefix);
        }
    }
    Ok(variants)
}

/// Adds a variant for each combination of one value of each subcategory of
/// the category that applies to the whole game (`level` is `None`) or to that
/// level. The first subcategory changes slowest.
fn push_variants(
    variants: &mut Vec<Variant>,
    category: &RawCategory,
    level: Option<&str>,
    prefix: &str,
) {
    // Each combination so far: the chosen value of each variable, and labels.
    let mut combinations: Vec<(BTreeMap<String, String>, Vec<&str>)> =
        vec![(BTreeMap::new(), Vec::new())];
    let variables = category.variables.iter().flat_map(|v| &v.data);
    for variable in variables.filter(|v| v.is_subcategory && applies(&v.scope, level)) {
        if variable.values.values.0.is_empty() {
            continue;
        }
        combinations = combinations
            .iter()
            .flat_map(|(chosen, labels)| {
                variable.values.values.0.iter().map(move |(id, label)| {
                    let mut chosen = chosen.clone();
                    chosen.insert(variable.id.clone(), id.clone());
                    let mut labels = labels.clone();
                    labels.push(label);
                    (chosen, labels)
                })
            })
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
fn applies(scope: &RawScope, level: Option<&str>) -> bool {
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
    use super::*;

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
        let levels = format!(
            r#"{{"id":"bob","name":"Bob-omb Battlefield","categories":{{"data":[{stage}]}}}},
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
    }

    #[test]
    fn an_answer_without_data_is_an_error() {
        assert!(parse_variants(r#"{"status":420}"#).is_err());
        assert!(parse_variants("not json").is_err());
    }
}
