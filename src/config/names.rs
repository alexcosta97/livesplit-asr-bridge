//! Game names: the slug a game's file is named after, and the name the Game
//! dialog suggests for a new auto splitter.

/// The longest slug, in characters, so file names stay well within every
/// OS's limits.
const MAX_SLUG_CHARS: usize = 64;

/// Words that name the file as an auto splitter rather than the game, left
/// out of the suggested name when they end the file name.
const SPLITTER_WORDS: &[&str] = &["asr", "auto", "autosplitter", "autosplitters", "splitter"];

/// Names Windows reserves for devices, which can't be used as file names
/// even with an extension.
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A filename-safe slug of a game name, for its file in `games/`: lowercase
/// letters and digits, with a single `-` for each run of anything else.
/// Letters outside ASCII are kept, since every OS allows them in file names.
pub fn slug(name: &str) -> String {
    let mut slug = String::new();
    let mut chars = 0;
    let mut gap = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        if !c.is_alphanumeric() {
            gap = true;
            continue;
        }
        let dash = gap && !slug.is_empty();
        let added = if dash { 2 } else { 1 };
        if chars + added > MAX_SLUG_CHARS {
            break;
        }
        if dash {
            slug.push('-');
        }
        slug.push(c);
        chars += added;
        gap = false;
    }
    if slug.is_empty() {
        slug.push_str("game");
    } else if WINDOWS_RESERVED.contains(&slug.as_str()) {
        slug.push_str("-game");
    }
    slug
}

/// The game name the Game dialog suggests for an auto splitter's file: the
/// file name without its extension, with `_`, `-` and `.` read as spaces,
/// and without words like "autosplitter" at the end. For example
/// `gta_sa_de_autosplitter.wasm` suggests "gta sa de".
pub fn name_from_file(file_name: &str) -> String {
    let stem = file_name
        .strip_suffix(".wasm")
        .or_else(|| file_name.strip_suffix(".WASM"))
        .unwrap_or(file_name);
    let words: Vec<&str> = stem
        .split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace())
        .filter(|word| !word.is_empty())
        .collect();
    let mut kept = words.as_slice();
    while let [rest @ .., last] = kept {
        if !SPLITTER_WORDS.contains(&last.to_lowercase().as_str()) {
            break;
        }
        kept = rest;
    }
    // A file named only "autosplitter.wasm" keeps its words: an empty
    // suggestion helps nobody.
    if kept.is_empty() {
        kept = &words;
    }
    kept.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_lowercase_words_joined_by_dashes() {
        assert_eq!(
            slug("GTA San Andreas — Definitive Edition"),
            "gta-san-andreas-definitive-edition"
        );
        assert_eq!(slug("Portal 2"), "portal-2");
    }

    #[test]
    fn slug_drops_characters_unsafe_in_file_names() {
        assert_eq!(slug("Half-Life: Source"), "half-life-source");
        assert_eq!(slug("a/b\\c:d*e?f\"g<h>i|j"), "a-b-c-d-e-f-g-h-i-j");
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(slug("  spaced   out  "), "spaced-out");
    }

    #[test]
    fn slug_keeps_letters_outside_ascii() {
        assert_eq!(slug("Pokémon Émeraude"), "pokémon-émeraude");
        assert_eq!(slug("ゼルダの伝説"), "ゼルダの伝説");
    }

    #[test]
    fn slug_of_nothing_usable_is_game() {
        assert_eq!(slug(""), "game");
        assert_eq!(slug("!!!"), "game");
    }

    #[test]
    fn slug_avoids_names_windows_reserves() {
        assert_eq!(slug("CON"), "con-game");
        assert_eq!(slug("Nul"), "nul-game");
        assert_eq!(slug("Console"), "console");
    }

    #[test]
    fn slug_is_limited_in_length() {
        let long = "a".repeat(200);
        assert_eq!(slug(&long).chars().count(), MAX_SLUG_CHARS);
        let words = "word ".repeat(40);
        let slug = slug(&words);
        assert!(slug.chars().count() <= MAX_SLUG_CHARS);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn name_from_file_drops_the_extension_and_splitter_words() {
        assert_eq!(name_from_file("gta_sa_de_autosplitter.wasm"), "gta sa de");
        assert_eq!(name_from_file("Celeste-Auto-Splitter.wasm"), "Celeste");
        assert_eq!(name_from_file("hollow_knight_asr.wasm"), "hollow knight");
        assert_eq!(name_from_file("portal2.wasm"), "portal2");
    }

    #[test]
    fn name_from_file_keeps_splitter_words_that_are_all_there_is() {
        assert_eq!(name_from_file("autosplitter.wasm"), "autosplitter");
    }

    #[test]
    fn name_from_file_keeps_splitter_words_before_the_end() {
        assert_eq!(name_from_file("auto_racer.wasm"), "auto racer");
    }
}
