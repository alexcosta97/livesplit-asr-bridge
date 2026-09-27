//! Config: the app's files in the OS's standard per-user configuration
//! folder (spec §7.1).
//!
//! ```text
//! <config folder>/
//!   app.toml            the app's own settings, such as the server port
//!   splitters.toml      maps each known .wasm path to a game
//!   games/
//!     <game-slug>.toml  display name and saved auto splitter settings
//! ```
//!
//! Only the files that are needed are read: `app.toml` at start-up,
//! `splitters.toml` when an auto
//! splitter is loaded or its game changed, and a game's file when its
//! settings are needed. Every write replaces the file whole, through a
//! temporary file, so an interrupted write never leaves half a file.

mod app;
mod games;
mod names;
mod settings;
mod splitters;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::ui::APP_NAME;

pub use app::AppSettings;
pub use games::{Game, GameSummary};
pub use names::{name_from_file, slug};
#[expect(
    unused_imports,
    reason = "settings are saved from the draft, not the runtime"
)]
pub use settings::from_runtime;
pub use settings::{SettingKey, merge_saved, revert_to_defaults, to_runtime};
pub use splitters::Splitters;

/// The app's configuration folder and the files in it.
#[derive(Debug, Clone)]
pub struct Config {
    dir: PathBuf,
}

impl Config {
    /// The configuration folder in the OS's standard place for the current
    /// user: `$XDG_CONFIG_HOME/livesplit-asr-bridge` (default
    /// `~/.config/livesplit-asr-bridge`) on Linux,
    /// `~/Library/Application Support/livesplit-asr-bridge` on macOS and
    /// `%APPDATA%\livesplit-asr-bridge` on Windows. `None` if the OS doesn't
    /// say where that is, for example with no home folder.
    pub fn standard() -> Option<Self> {
        dirs::config_dir().map(|dir| Self::at(dir.join(APP_NAME)))
    }

    /// A configuration folder at `dir`, for example a temporary one in tests.
    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The configuration folder.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the Preferences tab shows it (#13)")
    )]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn app_file(&self) -> PathBuf {
        self.dir.join("app.toml")
    }

    fn splitters_file(&self) -> PathBuf {
        self.dir.join("splitters.toml")
    }

    fn games_dir(&self) -> PathBuf {
        self.dir.join("games")
    }

    fn game_file(&self, slug: &str) -> PathBuf {
        self.games_dir().join(format!("{slug}.toml"))
    }
}

/// Reads a file, or `None` if it doesn't exist.
fn read_if_exists(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("couldn't read {}: {error}", path.display())),
    }
}

/// Replaces a file with `text`, creating its folder if needed. The text is
/// written to a temporary file first, then moved over the file.
fn write_replacing(path: &Path, text: &str) -> Result<(), String> {
    let describe = |error: io::Error| format!("couldn't write {}: {error}", path.display());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(describe)?;
    }
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, text).map_err(describe)?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        describe(error)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_folder_is_named_after_the_app_in_the_os_config_folder() {
        // Where the OS keeps configuration is the `dirs` crate's job; this
        // checks the app's folder is inside it and named after the app.
        let Some(config) = Config::standard() else {
            return;
        };
        assert_eq!(
            config.dir(),
            dirs::config_dir().unwrap().join("livesplit-asr-bridge")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_folder_is_under_xdg_config_home() {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
        let expected = match (xdg, home) {
            (Some(xdg), _) if xdg.is_absolute() => xdg,
            (_, Some(home)) => home.join(".config"),
            _ => return,
        };
        assert_eq!(
            Config::standard().unwrap().dir(),
            expected.join("livesplit-asr-bridge")
        );
    }

    #[test]
    fn files_are_where_the_spec_puts_them() {
        let config = Config::at(PathBuf::from("/config"));
        assert_eq!(config.splitters_file(), Path::new("/config/splitters.toml"));
        assert_eq!(
            config.game_file("gta-san-andreas"),
            Path::new("/config/games/gta-san-andreas.toml")
        );
    }

    #[test]
    fn writing_creates_the_folder_and_replaces_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("file.toml");
        write_replacing(&path, "a = 1\n").unwrap();
        write_replacing(&path, "a = 2\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "a = 2\n");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn a_missing_file_reads_as_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_if_exists(&dir.path().join("missing")), Ok(None));
    }
}
