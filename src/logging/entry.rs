//! One logged line and its category (spec §8.1).

use chrono::NaiveDateTime;

/// What a line is about. Errors are always shown; the others are shown
/// when their filter is ticked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Load failures, auto splitter crashes, server start failures and
    /// connection errors.
    Error,
    /// Messages printed by the auto splitter, and its actions on the timer.
    AutoSplitter,
    /// Timers connecting and disconnecting, commands sent, events and
    /// responses received, and dropped commands.
    Connection,
    /// Process attach and detach, tick rate changes, settings saved, server
    /// restarts and reloads.
    App,
}

impl Category {
    /// The tag in front of the line, as in the mockups.
    pub fn tag(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::AutoSplitter => "AUTO SPLITTER",
            Self::Connection => "CONNECTION",
            Self::App => "APP",
        }
    }
}

/// One logged line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Tells entries apart, for example to highlight one.
    pub id: u64,
    /// When it was recorded, in local time.
    pub time: NaiveDateTime,
    pub category: Category,
    pub message: String,
}

impl Entry {
    /// The time as shown and written: hours to milliseconds.
    pub fn clock(&self) -> String {
        self.time.format("%H:%M:%S%.3f").to_string()
    }

    /// The line as written to disk and exported: time, tag and message,
    /// with a newline.
    pub fn line(&self) -> String {
        format!(
            "{}  {}  {}\n",
            self.clock(),
            self.category.tag(),
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn a_line_is_time_tag_and_message() {
        let entry = Entry {
            id: 0,
            time: NaiveDate::from_ymd_opt(2026, 9, 27)
                .unwrap()
                .and_hms_milli_opt(19, 31, 1, 254)
                .unwrap(),
            category: Category::AutoSplitter,
            message: "Split: Los Santos — Gym Moves".to_owned(),
        };
        assert_eq!(
            entry.line(),
            "19:31:01.254  AUTO SPLITTER  Split: Los Santos — Gym Moves\n"
        );
    }
}
