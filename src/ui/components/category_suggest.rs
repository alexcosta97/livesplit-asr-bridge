//! The speedrun.com categories a Category dialog name field offers (spec
//! §6.8, §7.5.1): which variants the list shows for what is typed, the
//! field's link, and the list under the field.

use std::collections::HashSet;

use eframe::egui::{self, Color32, Id, Key, Order, Rect, ScrollArea, Sense, Ui, pos2, vec2};

use super::{
    checkbox::Checkbox,
    game_dialog::{RESULT_HEIGHT, RESULTS_SHOWN, linked_line, list_frame, one_line, typed_row},
};
use crate::{
    speedrun::{CategoryLink, Variant},
    ui::theme,
};

/// Up to this many variants, the list shows them all with the field empty.
const FEW: usize = 20;

/// What the list shows for what is typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rows {
    /// Too many variants to list with the field empty: one muted row,
    /// "Type to search <count> categories".
    TypeToSearch(usize),
    /// The variants listed, by their index in the game's variants.
    Variants(Vec<usize>),
}

/// The state of one open name field: its link and its list.
#[derive(Debug, Clone, Default)]
pub struct Suggest {
    /// The variant the name is linked to.
    pub link: Option<CategoryLink>,
    /// Whether the list is showing.
    pub list_open: bool,
    /// The highlighted row; the last row's index is the number of variants
    /// listed.
    pub highlight: Option<usize>,
    /// Whether the highlight moved with the keyboard and should be scrolled
    /// to.
    pub highlight_moved: bool,
    /// Whether the level variants are offered too.
    pub levels: bool,
    /// Whether the name in the field wasn't typed, so a clear match links it
    /// without asking, once the variants are in.
    pub may_link_on_own: bool,
    /// Names kept unlinked with the list's last row, by [`key`]: they aren't
    /// linked on their own again while the dialog is open.
    pub declined: HashSet<String>,
}

impl Suggest {
    /// A field opening with `link`, its list open unless it is linked. The
    /// level variants are offered from the start for a level variant's link.
    /// `declined` carries the names declined in the dialog's earlier fields.
    pub fn open(
        link: Option<CategoryLink>,
        may_link_on_own: bool,
        declined: HashSet<String>,
    ) -> Self {
        Self {
            list_open: link.is_none(),
            levels: link.as_ref().is_some_and(|link| link.level.is_some()),
            link,
            highlight: None,
            highlight_moved: false,
            may_link_on_own,
            declined,
        }
    }

    /// The variants offered for `typed`: the full-game ones, and the level
    /// ones while `levels` is ticked. With the field empty, all of them if
    /// there are a few, or else a row asking to type; otherwise the ones whose
    /// name contains what is typed, comparing letters and digits only and
    /// ignoring case.
    pub fn rows(variants: &[Variant], levels: bool, typed: &str) -> Rows {
        let offered = offered(variants, levels);
        if typed.trim().is_empty() {
            return if offered.len() <= FEW {
                Rows::Variants(offered)
            } else {
                Rows::TypeToSearch(offered.len())
            };
        }
        let typed = normalize(typed);
        Rows::Variants(
            offered
                .into_iter()
                .filter(|&index| normalize(&variants[index].name).contains(&typed))
                .collect(),
        )
    }

    /// The name was typed: the link goes, the list opens for it, and it is
    /// never linked on its own.
    pub fn typed(&mut self) {
        self.link = None;
        self.may_link_on_own = false;
        self.open_list();
    }

    /// A variant was picked, or matched clearly: the name is linked to it.
    pub fn pick(&mut self, link: CategoryLink) {
        self.link = Some(link);
        self.may_link_on_own = false;
        self.close_list();
    }

    /// The list's last row: keeps `name` unlinked, and doesn't link it on its
    /// own again.
    pub fn pick_typed(&mut self, name: &str) {
        self.declined.insert(key(name));
        self.link = None;
        self.may_link_on_own = false;
        self.close_list();
    }

    /// Whether `name` was kept unlinked with the last row.
    pub fn declined(&self, name: &str) -> bool {
        self.declined.contains(&key(name))
    }

    /// **Change** on the linked line: removes the link and opens the list.
    pub fn change_link(&mut self) {
        self.link = None;
        self.may_link_on_own = false;
        self.open_list();
    }

    /// The level box was ticked or unticked: the rows change under the
    /// highlight.
    pub fn levels_changed(&mut self) {
        self.highlight = None;
    }

    fn open_list(&mut self) {
        self.list_open = true;
        self.highlight = None;
    }

    pub fn close_list(&mut self) {
        self.list_open = false;
        self.highlight = None;
    }

    /// A key pressed while the list, of `rows` rows counting the last one, is
    /// open: Up and Down move the highlight, Escape closes the list, and Enter
    /// is used when a row is highlighted, to pick it. Returns whether the key
    /// was used, so it can be kept from the field and the dialog.
    pub fn key(&mut self, key: Key, rows: usize) -> bool {
        if !self.list_open {
            return false;
        }
        match key {
            Key::ArrowDown if rows > 0 => {
                self.highlight = Some(self.highlight.map_or(0, |row| (row + 1).min(rows - 1)));
                self.highlight_moved = true;
            }
            Key::ArrowUp if rows > 0 => {
                self.highlight = Some(self.highlight.map_or(0, |row| row.saturating_sub(1)));
                self.highlight_moved = true;
            }
            Key::ArrowDown | Key::ArrowUp => {}
            Key::Enter => return self.highlight.is_some(),
            Key::Escape => self.close_list(),
            _ => return false,
        }
        true
    }
}

/// The indices of the variants offered: the full-game ones, and the level
/// ones when `levels`.
pub fn offered(variants: &[Variant], levels: bool) -> Vec<usize> {
    variants
        .iter()
        .enumerate()
        .filter(|(_, variant)| levels || !variant.is_level())
        .map(|(index, _)| index)
        .collect()
}

/// `text` in lower case with only its letters and digits.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// How declined names are keyed: trimmed and in lower case.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// A variant row as the list shows it.
#[derive(Debug, Clone, Copy)]
pub struct Row<'a> {
    pub name: &'a str,
    /// A category of the game is linked to it.
    pub set_up: bool,
    /// It can't be picked: another category has it.
    pub dimmed: bool,
}

/// What the list shows, ready to draw.
pub enum Shown<'a> {
    /// "Type to search <count> categories".
    TypeToSearch(usize),
    /// The variants listed.
    Variants(Vec<Row<'a>>),
}

/// What was clicked in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clicked {
    /// The variant row at this index.
    Row(usize),
    /// The last row, which keeps the typed name.
    Typed,
}

/// What happened under the field this frame.
#[derive(Debug, Default)]
pub struct Under {
    /// A row of the list was clicked.
    pub clicked: Option<Clicked>,
    /// A click under the field took its focus, which it should get back.
    pub refocus: bool,
}

/// Draws what goes under the name field `field` of a linked game: the linked
/// line while it is linked, the level box while `has_levels`, and the list,
/// `width` wide, dropping down below them over what is under them. Its last
/// row keeps `typed`, once something is typed. A click outside the field,
/// the list and what is under the field closes the list.
pub fn under_field(
    ui: &mut Ui,
    field: Rect,
    width: f32,
    suggest: &mut Suggest,
    has_levels: bool,
    shown: &Shown<'_>,
    typed: &str,
) -> Under {
    let mut under = Under::default();
    if suggest.link.is_some() && linked_line(ui) {
        suggest.change_link();
        under.refocus = true;
    }
    if has_levels {
        ui.add_space(8.0);
        let levels = ui.add(Checkbox::new(
            &mut suggest.levels,
            "Include level categories",
        ));
        if levels.changed() {
            suggest.levels_changed();
            under.refocus = true;
        }
    }
    let below = Rect::from_min_max(
        field.min,
        pos2(
            field.left() + width,
            ui.min_rect().bottom().max(field.bottom()),
        ),
    );
    let typed = typed.trim();
    let empty = matches!(shown, Shown::Variants(rows) if rows.is_empty()) && typed.is_empty();
    if !suggest.list_open || empty {
        return under;
    }
    let area = egui::Area::new(Id::new("category dialog suggestions"))
        .order(Order::Tooltip)
        .fixed_pos(pos2(field.left(), below.bottom() + 4.0))
        .show(ui.ctx(), |ui| {
            list_frame()
                .show(ui, |ui| list(ui, width - 2.0, suggest, shown, typed))
                .inner
        });
    match area.inner {
        Some(clicked) => {
            under.clicked = Some(clicked);
            under.refocus = true;
        }
        None => {
            let outside = ui.input(|input| {
                input.pointer.any_pressed()
                    && input.pointer.interact_pos().is_some_and(|pos| {
                        !area.response.rect.contains(pos) && !below.contains(pos)
                    })
            });
            if outside {
                suggest.close_list();
            }
        }
    }
    under
}

/// The rows of the list: the variants, five shown and scrolling for more,
/// then the last row; or the muted row asking to type.
fn list(
    ui: &mut Ui,
    width: f32,
    suggest: &mut Suggest,
    shown: &Shown<'_>,
    typed: &str,
) -> Option<Clicked> {
    ui.set_width(width);
    ui.spacing_mut().item_spacing.y = 0.0;
    let rows = match shown {
        Shown::TypeToSearch(count) => {
            muted_row(ui, width, format!("Type to search {count} categories"));
            return None;
        }
        Shown::Variants(rows) => rows,
    };
    let scroll_to = suggest.highlight.filter(|_| suggest.highlight_moved);
    suggest.highlight_moved = false;
    let mut clicked = None;
    if !rows.is_empty() {
        ScrollArea::vertical()
            .max_height(RESULTS_SHOWN * RESULT_HEIGHT)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for (index, row) in rows.iter().enumerate() {
                    let highlighted = suggest.highlight == Some(index);
                    let response = variant_row(ui, width, row, highlighted);
                    if scroll_to == Some(index) {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        clicked = Some(Clicked::Row(index));
                    }
                }
            });
    }
    if !typed.is_empty() {
        if !rows.is_empty() {
            let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
            ui.painter().rect_filled(line, 0.0, theme::BORDER);
        }
        let text = format!("Use \"{typed}\" (not on speedrun.com)");
        if typed_row(ui, width, text, suggest.highlight == Some(rows.len())) {
            clicked = Some(Clicked::Typed);
        }
    }
    clicked
}

/// A variant in the list: its name, and SET UP when a category of the game
/// is linked to it. A dimmed row is muted and can't be clicked.
fn variant_row(ui: &mut Ui, width: f32, row: &Row<'_>, highlighted: bool) -> egui::Response {
    let sense = if row.dimmed {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, RESULT_HEIGHT), sense);
    if ui.is_rect_visible(rect) {
        if highlighted || (!row.dimmed && response.hovered()) {
            ui.painter().rect_filled(rect, 0.0, theme::BORDER);
        }
        let (text, tag): (Color32, Color32) = if row.dimmed {
            (theme::TEXT_MUTED, theme::TEXT_MUTED)
        } else {
            (theme::TEXT, theme::TEXT_SECONDARY)
        };
        let mut right = rect.right() - 12.0;
        if row.set_up {
            let galley = ui
                .painter()
                .layout_no_wrap("SET UP".to_owned(), theme::mono(11.0), tag);
            let pos = pos2(
                right - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            right = pos.x - 12.0;
            ui.painter().galley(pos, galley, tag);
        }
        let name = one_line(
            ui,
            row.name.to_owned(),
            theme::body(13.0),
            text,
            (right - rect.left() - 12.0).max(0.0),
        );
        let pos = pos2(rect.left() + 12.0, rect.center().y - name.size().y / 2.0);
        ui.painter().galley(pos, name, text);
    }
    response
}

/// A muted row that can't be clicked, like "Type to search 214 categories".
fn muted_row(ui: &mut Ui, width: f32, text: String) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, RESULT_HEIGHT), Sense::hover());
    let galley = one_line(ui, text, theme::body(13.0), theme::TEXT_MUTED, width - 24.0);
    let pos = pos2(rect.left() + 12.0, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, theme::TEXT_MUTED);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(name: &str, level: bool) -> Variant {
        Variant {
            name: name.to_owned(),
            link: CategoryLink {
                id: name.to_owned(),
                values: Default::default(),
                level: level.then(|| "level".to_owned()),
            },
        }
    }

    /// `count` full-game variants; every third one is a PC variant.
    fn full_game(count: usize) -> Vec<Variant> {
        (0..count)
            .map(|i| {
                let platform = if i % 3 == 0 { "P-C" } else { "N64" };
                variant(&format!("Run {i} ({platform})"), false)
            })
            .collect()
    }

    fn len(rows: Rows) -> usize {
        match rows {
            Rows::Variants(rows) => rows.len(),
            Rows::TypeToSearch(_) => panic!("expected variants, got {rows:?}"),
        }
    }

    #[test]
    fn a_few_variants_are_listed_with_the_field_empty() {
        assert_eq!(
            Suggest::rows(&full_game(3), false, ""),
            Rows::Variants(vec![0, 1, 2])
        );
        assert_eq!(len(Suggest::rows(&full_game(20), false, " ")), 20);
        assert_eq!(
            Suggest::rows(&full_game(21), false, ""),
            Rows::TypeToSearch(21)
        );
        let pc = (0..21).filter(|i| i % 3 == 0).collect::<Vec<_>>();
        assert_eq!(
            Suggest::rows(&full_game(21), false, "pc"),
            Rows::Variants(pc.clone())
        );
        assert_eq!(
            Suggest::rows(&full_game(21), false, " P.c"),
            Rows::Variants(pc)
        );
    }

    #[test]
    fn level_variants_appear_only_while_ticked_and_count_towards_twenty() {
        let mut variants = full_game(19);
        variants.extend((0..5).map(|i| variant(&format!("Stage {i}: Any%"), true)));
        assert_eq!(len(Suggest::rows(&variants, false, "")), 19);
        assert_eq!(Suggest::rows(&variants, true, ""), Rows::TypeToSearch(24));
        assert_eq!(
            Suggest::rows(&variants, true, "stage 4"),
            Rows::Variants(vec![23])
        );
        assert_eq!(len(Suggest::rows(&variants, false, "stage")), 0);
    }

    #[test]
    fn keys_move_the_highlight_and_escape_closes_the_list() {
        let mut suggest = Suggest::open(None, false, HashSet::new());
        assert!(suggest.list_open);
        assert!(
            !suggest.key(Key::Enter, 3),
            "Enter without a highlight is the field's"
        );
        assert!(suggest.key(Key::ArrowDown, 3));
        assert!(suggest.key(Key::ArrowDown, 3));
        assert!(suggest.key(Key::ArrowDown, 3));
        assert!(suggest.key(Key::ArrowDown, 3));
        assert_eq!(suggest.highlight, Some(2), "the last row is the last stop");
        assert!(suggest.key(Key::ArrowUp, 3));
        assert_eq!(suggest.highlight, Some(1));
        assert!(suggest.key(Key::Enter, 3));
        assert!(!suggest.key(Key::Tab, 3));
        assert!(suggest.key(Key::Escape, 3));
        assert!(!suggest.list_open);
        assert!(
            !suggest.key(Key::Escape, 3),
            "a second Escape is the dialog's"
        );
    }

    #[test]
    fn a_linked_field_opens_closed_and_a_level_link_ticks_the_box() {
        let link = variant("Stage: Any%", true).link;
        let suggest = Suggest::open(Some(link), false, HashSet::new());
        assert!(!suggest.list_open);
        assert!(suggest.levels);
    }
}
