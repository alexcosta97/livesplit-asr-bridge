//! speedrun.com game suggestions: the games of a search, and how a name is
//! matched against them (spec §7.5).

mod categories;
mod matching;
mod search;
#[cfg(test)]
mod test_server;

#[cfg(not(test))]
#[allow(unused_imports)]
// Used by the Category dialog, which comes later.
pub use categories::live_categories;
#[allow(unused_imports)]
// Used by the Category dialog, which comes later.
pub use categories::{CategoryLink, CategorySource, Variant, parse_variants};
#[cfg(test)]
#[allow(unused_imports)]
// Used by the tests of the Category dialog, which comes later.
pub use categories::{FakeCategories, OfflineCategories};
pub use matching::clear_match;
#[allow(unused_imports)]
// Used by the Category dialog, which comes later.
pub use matching::clear_variant;
pub use search::GameSearch;
#[cfg(not(test))]
pub use search::live;
#[cfg(test)]
pub use search::{FakeSearch, Offline};

use serde::Deserialize;

/// A game as speedrun.com lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedrunGame {
    /// speedrun.com's id for the game.
    pub id: String,
    /// The game's international name.
    pub name: String,
    /// The short name used in the game's address; empty when it has none.
    pub abbreviation: String,
    /// The year the game was released, when known.
    pub year: Option<u16>,
}

/// The link between a game here and its speedrun.com page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeedrunLink {
    /// speedrun.com's id for the game.
    pub id: String,
    /// The game's name on speedrun.com when the link was made.
    pub name: String,
}

impl From<&SpeedrunGame> for SpeedrunLink {
    fn from(game: &SpeedrunGame) -> Self {
        Self {
            id: game.id.clone(),
            name: game.name.clone(),
        }
    }
}

/// The body of a games search, as speedrun.com sends it.
#[derive(Deserialize)]
struct Response {
    data: Vec<RawGame>,
}

/// One game of a search response.
#[derive(Deserialize)]
struct RawGame {
    id: String,
    names: RawNames,
    abbreviation: Option<String>,
    released: Option<u16>,
}

/// The names of a game in a search response.
#[derive(Deserialize)]
struct RawNames {
    international: String,
}

/// Reads the games out of the body of a games search.
pub fn parse_games(json: &str) -> Result<Vec<SpeedrunGame>, String> {
    let response: Response = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(response
        .data
        .into_iter()
        .map(|game| SpeedrunGame {
            id: game.id,
            name: game.names.international,
            abbreviation: game.abbreviation.unwrap_or_default(),
            year: game.released,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_games_of_a_search() {
        let json = r#"{"data":[
          {"id":"4pd0n31e","names":{"international":"Portal","japanese":null},"abbreviation":"portal","released":2007},
          {"id":"x","names":{"international":"Odd"},"abbreviation":null,"released":null}],"pagination":{}}"#;
        assert_eq!(
            parse_games(json).unwrap(),
            vec![
                SpeedrunGame {
                    id: "4pd0n31e".into(),
                    name: "Portal".into(),
                    abbreviation: "portal".into(),
                    year: Some(2007)
                },
                SpeedrunGame {
                    id: "x".into(),
                    name: "Odd".into(),
                    abbreviation: String::new(),
                    year: None
                },
            ]
        );
    }

    #[test]
    fn a_response_without_data_is_an_error() {
        assert!(parse_games(r#"{"status":420}"#).is_err());
        assert!(parse_games("not json").is_err());
    }
}
