//! The UI components of spec §6.11, one per file, each implemented once.
//! Their values come from the Superdesign component templates and
//! `.superdesign/design-system.md`.

mod app_header;
mod button;
mod card;
mod error_card;
mod game_card;
mod game_dialog;
mod section_label;
mod splitter_card;
mod splitter_strip;
mod status_word;

pub use app_header::app_header;
pub use error_card::{ErrorAction, error_card};
pub use game_card::game_card;
pub use game_dialog::{GameChoice, GameDialog, GameDialogAction, GameDialogKind, game_dialog};
pub use section_label::section_label;
pub use splitter_card::{SplitterAction, splitter_card};
pub use splitter_strip::splitter_strip;

use eframe::egui::Response;

/// Whether the window is wide (status column and tab area) or compact
/// (status column only, bigger status words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Wide,
    Compact,
}

/// The tooltip of every Reload button.
fn reload_hint(response: Response) -> Response {
    response.on_hover_text("Load the same file again, for example after rebuilding it")
}
