//! The in-app view: the most recent lines, and which of them are shown.

use std::collections::VecDeque;

use super::{Category, Entry};

/// How many lines the in-app view keeps (spec §8.1).
pub const CAPACITY: usize = 10_000;

/// Which optional categories are shown. Errors are always shown. By default
/// only errors are.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Filters {
    pub auto_splitter: bool,
    pub connection: bool,
    pub app: bool,
}

impl Filters {
    /// Whether lines of `category` are shown.
    pub fn shows(self, category: Category) -> bool {
        match category {
            Category::Error => true,
            Category::AutoSplitter => self.auto_splitter,
            Category::Connection => self.connection,
            Category::App => self.app,
        }
    }
}

/// The most recent lines of every category, oldest first.
#[derive(Debug, Default)]
pub struct LogView {
    entries: VecDeque<Entry>,
}

impl LogView {
    /// Adds a line, dropping the oldest one when full.
    pub fn push(&mut self, entry: Entry) {
        if self.entries.len() == CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Removes every line.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// How many lines are kept, shown or not.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The lines `filters` show, oldest first.
    pub fn shown(&self, filters: Filters) -> impl Iterator<Item = &Entry> {
        self.entries
            .iter()
            .filter(move |entry| filters.shows(entry.category))
    }

    /// The lines `filters` show, as text to copy or save.
    pub fn export(&self, filters: Filters) -> String {
        self.shown(filters).map(Entry::line).collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    fn entry(id: u64, category: Category) -> Entry {
        Entry {
            id,
            time: NaiveDateTime::default(),
            category,
            message: format!("line {id}"),
        }
    }

    fn filled() -> LogView {
        let mut view = LogView::default();
        view.push(entry(0, Category::AutoSplitter));
        view.push(entry(1, Category::Error));
        view.push(entry(2, Category::Connection));
        view.push(entry(3, Category::App));
        view
    }

    fn ids(view: &LogView, filters: Filters) -> Vec<u64> {
        view.shown(filters).map(|entry| entry.id).collect()
    }

    #[test]
    fn by_default_only_errors_are_shown() {
        assert_eq!(ids(&filled(), Filters::default()), [1]);
    }

    #[test]
    fn ticking_a_category_shows_its_earlier_lines() {
        let view = filled();
        let filters = Filters {
            auto_splitter: true,
            app: true,
            ..Filters::default()
        };
        assert_eq!(ids(&view, filters), [0, 1, 3]);
    }

    #[test]
    fn errors_are_shown_whatever_the_filters() {
        let view = filled();
        let all = Filters {
            auto_splitter: true,
            connection: true,
            app: true,
        };
        assert_eq!(ids(&view, all), [0, 1, 2, 3]);
    }

    #[test]
    fn export_has_only_the_lines_shown() {
        let view = filled();
        let filters = Filters {
            connection: true,
            ..Filters::default()
        };
        assert_eq!(
            view.export(filters),
            "00:00:00.000  ERROR  line 1\n00:00:00.000  CONNECTION  line 2\n"
        );
    }

    #[test]
    fn keeps_the_most_recent_lines() {
        let mut view = LogView::default();
        for id in 0..CAPACITY as u64 + 5 {
            view.push(entry(id, Category::Error));
        }
        assert_eq!(view.len(), CAPACITY);
        assert_eq!(view.shown(Filters::default()).next().unwrap().id, 5);
    }

    #[test]
    fn clear_empties_the_view() {
        let mut view = filled();
        view.clear();
        assert_eq!(view.len(), 0);
    }
}
