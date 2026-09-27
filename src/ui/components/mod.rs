//! The UI components of spec §6.11, one per file, each implemented once.
//! Their values come from the Superdesign component templates and
//! `.superdesign/design-system.md`.

mod address_row;
mod app_header;
mod button;
mod card;
mod checkbox;
mod empty_state;
mod error_card;
mod game_card;
mod game_dialog;
mod how_to_connect;
mod info_marker;
mod log_line;
mod log_toolbar;
mod section_label;
mod server_section;
mod setting_choice;
mod setting_heading;
mod settings_toolbar;
mod splitter_card;
mod splitter_strip;
mod status_word;
mod tab_strip;
mod timer_card;
mod timer_row;
mod unsaved_dialog;

pub use app_header::app_header;
pub use checkbox::Checkbox;
pub use empty_state::empty_state;
pub use error_card::{ErrorAction, error_card};
pub use game_card::game_card;
pub use game_dialog::{GameChoice, GameDialog, GameDialogAction, GameDialogKind, game_dialog};
pub use how_to_connect::how_to_connect;
pub use info_marker::info_marker;
pub use log_line::{LOG_LINE_HEIGHT, log_line};
pub use log_toolbar::{LogAction, log_toolbar};
pub use section_label::section_label;
pub use server_section::{PortNote, server_section};
pub use setting_choice::{setting_choice, setting_file, setting_text};
pub use setting_heading::setting_heading;
pub use settings_toolbar::{SaveStatus, ToolbarAction, settings_toolbar};
pub use splitter_card::{SplitterAction, splitter_card};
pub use splitter_strip::splitter_strip;
pub use tab_strip::tab_strip;
pub use timer_card::timer_card;
pub use timer_row::timer_row;
pub use unsaved_dialog::{UnsavedAction, UnsavedTrigger, unsaved_dialog};

use eframe::egui::{Color32, Response};

/// The flat overlay that dims the window behind a dialog: 60 % black.
const BACKDROP: Color32 = Color32::from_black_alpha(153);

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
