//! Matching a game's name against speedrun.com games (spec §7.5).

use super::{SpeedrunGame, Variant};

/// The score from which a game counts as a clear match for a name.
pub const CLEAR_MATCH: f64 = 0.9;

/// Lowercases `text` and keeps only its letters and digits, so punctuation,
/// spaces and case do not matter when comparing names.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// How well `name` matches `candidate`, from 0 to 1, both normalized.
fn similarity(name: &str, candidate: &str) -> f64 {
    if name.is_empty() || candidate.is_empty() {
        0.0
    } else if name == candidate {
        1.0
    } else {
        strsim::normalized_levenshtein(name, candidate)
    }
}

/// How well `name` matches a game, from 0 to 1: the better of its match with
/// the game's name and with its abbreviation. A name with no letters or digits
/// matches nothing.
pub fn score(name: &str, game: &SpeedrunGame) -> f64 {
    let name = normalize(name);
    similarity(&name, &normalize(&game.name)).max(similarity(&name, &normalize(&game.abbreviation)))
}

/// The index of the one game that clearly matches `name`, or `None` when no
/// game or more than one game matches clearly.
pub fn clear_match(name: &str, games: &[SpeedrunGame]) -> Option<usize> {
    let mut clear = games
        .iter()
        .enumerate()
        .filter(|(_, game)| score(name, game) >= CLEAR_MATCH)
        .map(|(index, _)| index);
    let first = clear.next()?;
    clear.next().is_none().then_some(first)
}

/// The index of the one variant whose name clearly matches `name`, or `None`
/// when no variant or more than one variant matches clearly.
pub fn clear_variant(name: &str, variants: &[&Variant]) -> Option<usize> {
    let name = normalize(name);
    let mut clear = variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| similarity(&name, &normalize(&variant.name)) >= CLEAR_MATCH)
        .map(|(index, _)| index);
    let first = clear.next()?;
    clear.next().is_none().then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(name: &str, abbreviation: &str) -> SpeedrunGame {
        SpeedrunGame {
            id: abbreviation.into(),
            name: name.into(),
            abbreviation: abbreviation.into(),
            year: None,
        }
    }

    #[test]
    fn same_letters_and_digits_match_fully() {
        assert_eq!(
            score(
                "grand theft auto san andreas",
                &game("Grand Theft Auto: San Andreas", "gtasa")
            ),
            1.0
        );
        assert_eq!(
            score("GTA-SA", &game("Grand Theft Auto: San Andreas", "gtasa")),
            1.0
        );
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
        let games = [
            game("Portal", "portal"),
            game("Portals", "portals"),
            game("Portal 2", "portal_2"),
        ];
        assert_eq!(clear_match("portal", &games), Some(0));
    }

    #[test]
    fn two_clear_matches_or_none_pick_nothing() {
        let games = [game("Mario Kart", "mk"), game("Mario Kart DS", "mariokart")];
        assert_eq!(clear_match("mario kart", &games), None);
        assert_eq!(
            clear_match(
                "gta sa de",
                &[game("Grand Theft Auto: San Andreas", "gtasa")]
            ),
            None
        );
        assert_eq!(clear_match("portal", &[]), None);
    }

    fn variant(name: &str) -> Variant {
        Variant {
            name: name.into(),
            link: Default::default(),
        }
    }

    #[test]
    fn a_variant_matches_clearly_by_its_name_only() {
        let pc = variant("Any% (PC)");
        let n64 = variant("Any% (N64)");
        let both = [&pc, &n64];
        assert_eq!(clear_variant("any%(pc)", &both), Some(0));
        assert_eq!(clear_variant("Any%", &both), None);
        assert_eq!(clear_variant("!!!", &both), None);
    }
}
