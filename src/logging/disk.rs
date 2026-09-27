//! The log files: one per day, kept for 7 days, with a per-day size cap
//! (spec §8.2).
//!
//! Every category is written, whatever the Log tab's filters. Past the cap,
//! only errors are written, after one line saying the cap was reached.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use chrono::NaiveDate;

use super::{Category, Entry};

/// The start of every log file's name, before the date.
const PREFIX: &str = "livesplit-asr-bridge-";
/// The end of every log file's name, after the date.
const SUFFIX: &str = ".log";
/// The date in a log file's name.
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Files more than this many days older than today are deleted.
const RETENTION_DAYS: i64 = 7;
/// The size past which only errors are written to a day's file: 50 MB.
const DAILY_CAP: u64 = 50 * 1024 * 1024;
/// The line written once when a day's file reaches the cap.
const CAP_MESSAGE: &str =
    "Today's log reached 50 MB, so only errors are written to it until tomorrow";

/// Writes lines to the day's file in the log folder.
pub struct DiskLog {
    dir: PathBuf,
    cap: u64,
    file: Option<DayFile>,
}

/// The open file of one day.
struct DayFile {
    date: NaiveDate,
    file: File,
    size: u64,
    /// Whether the cap was reached and the line saying so written.
    capped: bool,
}

impl DiskLog {
    /// A writer for the log folder `dir`, after deleting the files that are
    /// too old on `today`. Returns what couldn't be deleted.
    pub fn open(dir: PathBuf, today: NaiveDate) -> (Self, Vec<String>) {
        Self::with_cap(dir, today, DAILY_CAP)
    }

    fn with_cap(dir: PathBuf, today: NaiveDate, cap: u64) -> (Self, Vec<String>) {
        let log = Self {
            dir,
            cap,
            file: None,
        };
        let errors = log.delete_old(today);
        (log, errors)
    }

    /// Writes `entry` to its day's file, starting a new file when the day
    /// changes and then deleting the files that are too old. Returns what
    /// couldn't be deleted, or the error if the line couldn't be written.
    pub fn write(&mut self, entry: &Entry) -> Result<Vec<String>, String> {
        let date = entry.time.date();
        let mut errors = Vec::new();
        let file = match &mut self.file {
            Some(file) if file.date == date => file,
            current => {
                let rotating = current.is_some();
                *current = None;
                if rotating {
                    errors = self.delete_old(date);
                }
                self.file.insert(DayFile::open(&self.dir, date, self.cap)?)
            }
        };

        let line = entry.line();
        if file.size >= self.cap && entry.category != Category::Error {
            if !file.capped {
                let marker = Entry {
                    category: Category::App,
                    message: CAP_MESSAGE.to_owned(),
                    ..entry.clone()
                };
                file.append(&marker.line())?;
                file.capped = true;
            }
            return Ok(errors);
        }
        file.append(&line)?;
        Ok(errors)
    }

    /// Deletes the log files more than [`RETENTION_DAYS`] older than
    /// `today`. Other files in the folder are left alone. Returns what
    /// couldn't be deleted.
    fn delete_old(&self, today: NaiveDate) -> Vec<String> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Vec::new(),
            Err(error) => {
                return vec![format!(
                    "Couldn't delete old log files in {}: {error}",
                    self.dir.display()
                )];
            }
        };
        let mut errors = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(date) = entry.file_name().to_str().and_then(date_of) else {
                continue;
            };
            if (today - date).num_days() > RETENTION_DAYS
                && let Err(error) = fs::remove_file(&path)
            {
                errors.push(format!(
                    "Couldn't delete the old log file {}: {error}",
                    path.display()
                ));
            }
        }
        errors
    }
}

impl DayFile {
    /// Opens the file of `date` in `dir` to add to it, creating the folder
    /// and the file if needed.
    fn open(dir: &std::path::Path, date: NaiveDate, cap: u64) -> Result<Self, String> {
        let path = dir.join(file_name(date));
        let describe = |error: io::Error| format!("couldn't open {}: {error}", path.display());
        fs::create_dir_all(dir).map_err(describe)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)
            .map_err(describe)?;
        let size = file.metadata().map_err(describe)?.len();
        // The line saying the cap was reached is always after the cap, so
        // only that part of the file needs reading.
        let capped = size >= cap && {
            let mut rest = Vec::new();
            file.seek(SeekFrom::Start(cap)).map_err(describe)?;
            file.read_to_end(&mut rest).map_err(describe)?;
            String::from_utf8_lossy(&rest).contains(CAP_MESSAGE)
        };
        Ok(Self {
            date,
            file,
            size,
            capped,
        })
    }

    fn append(&mut self, line: &str) -> Result<(), String> {
        self.file
            .write_all(line.as_bytes())
            .map_err(|error| format!("couldn't write the log file: {error}"))?;
        self.size += line.len() as u64;
        Ok(())
    }
}

/// The name of the log file of `date`.
fn file_name(date: NaiveDate) -> String {
    format!("{PREFIX}{}{SUFFIX}", date.format(DATE_FORMAT))
}

/// The date of a log file from its name, or `None` if it isn't a log file.
fn date_of(name: &str) -> Option<NaiveDate> {
    let date = name.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    NaiveDate::parse_from_str(date, DATE_FORMAT).ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::Days;

    use super::*;

    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, n).unwrap()
    }

    fn entry(date: NaiveDate, category: Category, message: &str) -> Entry {
        Entry {
            id: 0,
            time: date.and_hms_milli_opt(19, 31, 1, 254).unwrap(),
            category,
            message: message.to_owned(),
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    fn read(dir: &Path, date: NaiveDate) -> String {
        fs::read_to_string(dir.join(file_name(date))).unwrap()
    }

    #[test]
    fn files_are_named_by_day() {
        assert_eq!(file_name(day(7)), "livesplit-asr-bridge-2026-09-07.log");
        assert_eq!(date_of("livesplit-asr-bridge-2026-09-07.log"), Some(day(7)));
        assert_eq!(date_of("livesplit-asr-bridge-2026-09-07.txt"), None);
        assert_eq!(date_of("livesplit-asr-bridge-latest.log"), None);
    }

    #[test]
    fn every_category_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let (mut log, _) = DiskLog::open(dir.path().to_owned(), day(27));
        for category in [
            Category::Error,
            Category::AutoSplitter,
            Category::Connection,
            Category::App,
        ] {
            log.write(&entry(day(27), category, "x")).unwrap();
        }
        assert_eq!(
            read(dir.path(), day(27)),
            "19:31:01.254  ERROR  x\n19:31:01.254  AUTO SPLITTER  x\n\
             19:31:01.254  CONNECTION  x\n19:31:01.254  APP  x\n"
        );
    }

    #[test]
    fn a_new_day_starts_a_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let (mut log, _) = DiskLog::open(dir.path().to_owned(), day(26));
        log.write(&entry(day(26), Category::App, "first")).unwrap();
        log.write(&entry(day(27), Category::App, "second")).unwrap();
        assert_eq!(
            names(dir.path()),
            [
                "livesplit-asr-bridge-2026-09-26.log",
                "livesplit-asr-bridge-2026-09-27.log"
            ]
        );
        assert_eq!(read(dir.path(), day(26)), "19:31:01.254  APP  first\n");
        assert_eq!(read(dir.path(), day(27)), "19:31:01.254  APP  second\n");
    }

    #[test]
    fn a_restart_adds_to_the_days_file() {
        let dir = tempfile::tempdir().unwrap();
        for message in ["before", "after"] {
            let (mut log, _) = DiskLog::open(dir.path().to_owned(), day(27));
            log.write(&entry(day(27), Category::App, message)).unwrap();
        }
        assert_eq!(
            read(dir.path(), day(27)),
            "19:31:01.254  APP  before\n19:31:01.254  APP  after\n"
        );
    }

    #[test]
    fn files_older_than_7_days_are_deleted_at_start_up() {
        let dir = tempfile::tempdir().unwrap();
        let today = day(27);
        for days_ago in [0, 1, 7, 8, 20] {
            let date = today.checked_sub_days(Days::new(days_ago)).unwrap();
            fs::write(dir.path().join(file_name(date)), "").unwrap();
        }
        fs::write(dir.path().join("notes.txt"), "").unwrap();
        let (_log, errors) = DiskLog::open(dir.path().to_owned(), today);
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            names(dir.path()),
            [
                "livesplit-asr-bridge-2026-09-20.log",
                "livesplit-asr-bridge-2026-09-26.log",
                "livesplit-asr-bridge-2026-09-27.log",
                "notes.txt"
            ]
        );
    }

    #[test]
    fn files_older_than_7_days_are_deleted_at_each_rotation() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(file_name(day(19))), "").unwrap();
        let (mut log, _) = DiskLog::open(dir.path().to_owned(), day(26));
        log.write(&entry(day(26), Category::App, "x")).unwrap();
        // Seven days old on the 26th: kept.
        assert!(dir.path().join(file_name(day(19))).exists());
        let errors = log.write(&entry(day(27), Category::App, "x")).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(!dir.path().join(file_name(day(19))).exists());
    }

    #[test]
    fn a_missing_folder_is_created_on_the_first_line() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("state").join("logs");
        let (mut log, errors) = DiskLog::open(logs.clone(), day(27));
        assert!(errors.is_empty(), "{errors:?}");
        log.write(&entry(day(27), Category::App, "x")).unwrap();
        assert_eq!(read(&logs, day(27)), "19:31:01.254  APP  x\n");
    }

    #[test]
    fn past_the_cap_only_errors_are_written_after_one_line_saying_so() {
        let dir = tempfile::tempdir().unwrap();
        // Each line is 25 bytes: the second line reaches the cap.
        let (mut log, _) = DiskLog::with_cap(dir.path().to_owned(), day(27), 50);
        log.write(&entry(day(27), Category::App, "one  ")).unwrap();
        log.write(&entry(day(27), Category::App, "two  ")).unwrap();
        log.write(&entry(day(27), Category::App, "dropped"))
            .unwrap();
        log.write(&entry(day(27), Category::Error, "kept")).unwrap();
        log.write(&entry(day(27), Category::Connection, "dropped"))
            .unwrap();
        log.write(&entry(day(27), Category::Error, "kept too"))
            .unwrap();
        assert_eq!(
            read(dir.path(), day(27)),
            format!(
                "19:31:01.254  APP  one  \n19:31:01.254  APP  two  \n\
                 19:31:01.254  APP  {CAP_MESSAGE}\n19:31:01.254  ERROR  kept\n\
                 19:31:01.254  ERROR  kept too\n"
            )
        );
    }

    #[test]
    fn the_cap_holds_after_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        for _ in 0..2 {
            let (mut log, _) = DiskLog::with_cap(dir.path().to_owned(), day(27), 50);
            for message in ["one  ", "two  ", "dropped"] {
                log.write(&entry(day(27), Category::App, message)).unwrap();
            }
        }
        let text = read(dir.path(), day(27));
        assert_eq!(text.matches(CAP_MESSAGE).count(), 1, "{text}");
        assert!(!text.contains("dropped"), "{text}");
    }

    #[test]
    fn the_cap_is_per_day() {
        let dir = tempfile::tempdir().unwrap();
        let (mut log, _) = DiskLog::with_cap(dir.path().to_owned(), day(26), 50);
        for message in ["one  ", "two  ", "dropped"] {
            log.write(&entry(day(26), Category::App, message)).unwrap();
        }
        log.write(&entry(day(27), Category::App, "written"))
            .unwrap();
        assert_eq!(read(dir.path(), day(27)), "19:31:01.254  APP  written\n");
    }

    #[test]
    fn the_real_cap_is_50_mb() {
        assert_eq!(DAILY_CAP, 50 * 1024 * 1024);
        assert!(CAP_MESSAGE.contains("50 MB"));
    }
}
