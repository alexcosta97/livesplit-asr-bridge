//! Logging: everything the app records, by category (spec §8).
//!
//! Every line goes to the in-app view, which keeps the most recent 10,000,
//! and to one file per day in the OS's log folder. The Log tab's filters
//! only change what the view shows, never what is recorded.

mod disk;
mod entry;
mod view;

use std::path::{Path, PathBuf};

use chrono::Local;

use crate::ui::APP_NAME;

pub use disk::DiskLog;
pub use entry::{Category, Entry};
pub use view::{Filters, LogView};

/// Records lines in the in-app view and on disk.
pub struct Logger {
    view: LogView,
    /// The log folder's writer, or `None` if there is no log folder or
    /// writing to it failed.
    disk: Option<DiskLog>,
    dir: Option<PathBuf>,
    next_id: u64,
}

impl Logger {
    /// A logger writing to `dir`, after deleting the files there that are
    /// too old to keep. With no folder, lines are only kept in the view.
    pub fn new(dir: Option<PathBuf>) -> Self {
        let mut logger = Self {
            view: LogView::default(),
            disk: None,
            dir: dir.clone(),
            next_id: 0,
        };
        match dir {
            None => logger.record(
                Category::Error,
                "Couldn't find the log folder, so the log isn't saved to disk".to_owned(),
            ),
            Some(dir) => {
                let (disk, errors) = DiskLog::open(dir, Local::now().date_naive());
                logger.disk = Some(disk);
                for error in errors {
                    logger.record(Category::Error, error);
                }
            }
        }
        logger
    }

    /// Records a line, stamped with the current time.
    pub fn record(&mut self, category: Category, message: String) {
        let entry = Entry {
            id: self.next_id,
            time: Local::now().naive_local(),
            category,
            message,
        };
        self.next_id += 1;
        let written = self.disk.as_mut().map(|disk| disk.write(&entry));
        self.view.push(entry);
        match written {
            None => {}
            Some(Ok(errors)) => {
                for error in errors {
                    self.record(Category::Error, error);
                }
            }
            Some(Err(error)) => {
                // Shown in the view only: writing it to disk would fail too.
                self.disk = None;
                self.record(
                    Category::Error,
                    format!(
                        "Couldn't write the log file, so the log isn't saved to disk until the app restarts: {error}"
                    ),
                );
            }
        }
    }

    /// The lines in the view.
    pub fn view(&self) -> &LogView {
        &self.view
    }

    /// Empties the in-app view. The files on disk are kept.
    pub fn clear(&mut self) {
        self.view.clear();
    }

    /// The log folder, if the OS has one for the user.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }
}

/// The log folder in the OS's standard place for the current user (spec
/// §8.2): `$XDG_STATE_HOME/livesplit-asr-bridge/logs` (default
/// `~/.local/state/livesplit-asr-bridge/logs`) on Linux,
/// `~/Library/Logs/livesplit-asr-bridge` on macOS and
/// `%LOCALAPPDATA%\livesplit-asr-bridge\logs` on Windows. `None` if the OS
/// doesn't say where that is, for example with no home folder.
pub fn standard_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library").join("Logs").join(APP_NAME))
    } else if cfg!(target_os = "linux") {
        dirs::state_dir().map(|dir| dir.join(APP_NAME).join("logs"))
    } else {
        // Windows, and other systems as a fallback.
        dirs::data_local_dir().map(|dir| dir.join(APP_NAME).join("logs"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_folder_is_under_xdg_state_home() {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let xdg = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from);
        let expected = match (xdg, home) {
            (Some(xdg), _) if xdg.is_absolute() => xdg,
            (_, Some(home)) => home.join(".local").join("state"),
            _ => return,
        };
        assert_eq!(
            standard_dir().unwrap(),
            expected.join("livesplit-asr-bridge").join("logs")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_folder_is_in_library_logs() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(
            standard_dir().unwrap(),
            home.join("Library/Logs/livesplit-asr-bridge")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_folder_is_in_local_app_data() {
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        assert_eq!(
            standard_dir().unwrap(),
            local.join("livesplit-asr-bridge").join("logs")
        );
    }

    #[test]
    fn lines_go_to_the_view_and_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut logger = Logger::new(Some(dir.path().to_owned()));
        logger.record(Category::AutoSplitter, "Split".to_owned());
        assert_eq!(logger.view().len(), 1);
        let file = std::fs::read_dir(dir.path()).unwrap().next().unwrap();
        let text = std::fs::read_to_string(file.unwrap().path()).unwrap();
        assert!(text.ends_with("  AUTO SPLITTER  Split\n"), "{text:?}");
    }

    #[test]
    fn a_failed_write_is_shown_once_and_stops_writing() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the log folder should be, so the folder can't be
        // created.
        let blocked = dir.path().join("logs");
        std::fs::write(&blocked, "").unwrap();
        let mut logger = Logger::new(Some(blocked));
        logger.record(Category::App, "one".to_owned());
        logger.record(Category::App, "two".to_owned());
        let errors = logger
            .view()
            .shown(Filters::default())
            .filter(|entry| entry.message.starts_with("Couldn't write the log file"))
            .count();
        assert_eq!(errors, 1);
        let lines: Vec<_> = logger
            .view()
            .shown(Filters {
                app: true,
                ..Filters::default()
            })
            .map(|entry| entry.message.as_str())
            .filter(|message| !message.starts_with("Couldn't"))
            .collect();
        assert_eq!(lines, ["one", "two"]);
    }

    #[test]
    fn with_no_folder_lines_stay_in_the_view() {
        let mut logger = Logger::new(None);
        logger.record(Category::App, "Loaded".to_owned());
        assert_eq!(logger.view().len(), 2);
    }
}
