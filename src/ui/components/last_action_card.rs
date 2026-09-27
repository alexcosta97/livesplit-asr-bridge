//! Last action card: the auto splitter's latest timer action, and the 2
//! before it.

use std::time::{Duration, Instant};

use eframe::egui::{Align, Label, Layout, RichText, Ui};

use super::{
    Width,
    card::{Edge, card},
    section_label::section_label,
    status_word::status_word,
};
use crate::ui::{
    last_action::{LastActions, ShownAction},
    theme,
};

/// How long the card's orange edge flashes when a new action arrives.
const FLASH: Duration = Duration::from_millis(300);

/// The Last action card: the latest action in large type with its time, a
/// note when it wasn't sent, and, when wide, the 2 previous actions as faint
/// lines; or the empty state. The orange edge flashes as an action arrives.
pub fn last_action_card(ui: &mut Ui, actions: &LastActions, width: Width, now: Instant) {
    let flashing = actions
        .latest()
        .map(|latest| FLASH.saturating_sub(now.saturating_duration_since(latest.arrived)))
        .filter(|left| !left.is_zero());
    if let Some(left) = flashing {
        ui.ctx().request_repaint_after(left);
    }
    let edge = flashing.map(|_| Edge {
        color: theme::ACCENT,
        tint: theme::WINDOW,
    });
    card(ui, edge, |ui| {
        section_label(ui, "Last action");
        ui.add_space(8.0);
        let Some(latest) = actions.latest() else {
            status_word(ui, None, "No actions yet", theme::TEXT_MUTED, width);
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Actions appear here when the auto splitter starts, splits or resets.",
                )
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
            );
            return;
        };
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            ui.label(time(latest, theme::TEXT_SECONDARY));
            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                status_word(ui, None, &latest.word, theme::ACCENT, width);
            });
        });
        if !latest.sent {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Not sent: no timer connected")
                    .font(theme::mono(12.0))
                    .color(theme::TEXT_MUTED),
            );
        }
        if width == Width::Wide {
            for (index, action) in actions.previous().enumerate() {
                ui.add_space(if index == 0 { 12.0 } else { 4.0 });
                ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                    ui.label(time(action, theme::TEXT_MUTED));
                    let word = RichText::new(action.word.to_uppercase())
                        .font(theme::mono(12.0))
                        .color(theme::TEXT_MUTED);
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        ui.add(Label::new(word).truncate());
                    });
                });
            }
        }
    });
}

/// An action's time of day, like `19:31:01`.
fn time(action: &ShownAction, color: eframe::egui::Color32) -> RichText {
    RichText::new(action.time.format("%H:%M:%S").to_string())
        .font(theme::mono(12.0))
        .color(color)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;
    use eframe::egui::{CentralPanel, Context, RawInput};

    use super::*;
    use crate::runner::{RunnerEvent, TimerAction};

    /// Lays out the card, returning the text it shows.
    fn shown(actions: &LastActions, width: Width, now: Instant) -> String {
        let ctx = Context::default();
        crate::ui::install_theme(&ctx);
        let mut output = ctx.run_ui(RawInput::default(), |ui| {
            CentralPanel::default().show(ui, |ui| {
                last_action_card(ui, actions, width, now);
            });
        });
        output.textures_delta.clear();
        let mut text = String::new();
        for shape in output.shapes {
            if let eframe::egui::Shape::Text(shape) = shape.shape {
                text.push_str(shape.galley.text());
                text.push('\n');
            }
        }
        text
    }

    fn actions(sent_to: usize) -> (LastActions, Instant) {
        let mut actions = LastActions::default();
        let now = Instant::now();
        for (index, action) in [TimerAction::Reset, TimerAction::Start, TimerAction::Split]
            .into_iter()
            .enumerate()
        {
            let time = NaiveTime::from_hms_opt(19, 31, index as u32).unwrap();
            actions.apply(&RunnerEvent::TimerAction { action, sent_to }, time, now);
        }
        (actions, now)
    }

    #[test]
    fn shows_the_empty_state() {
        let text = shown(&LastActions::default(), Width::Wide, Instant::now());
        assert!(text.contains("NO ACTIONS YET"), "{text}");
        assert!(text.contains("Actions appear here"), "{text}");
    }

    #[test]
    fn wide_shows_the_latest_action_and_the_two_before_it() {
        let (actions, now) = actions(1);
        let text = shown(&actions, Width::Wide, now);
        for expected in [
            "SPLIT", "19:31:02", "START", "19:31:01", "RESET", "19:31:00",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("Not sent"), "{text}");
    }

    #[test]
    fn compact_shows_only_the_latest_action() {
        let (actions, now) = actions(1);
        let text = shown(&actions, Width::Compact, now);
        assert!(text.contains("SPLIT"), "{text}");
        assert!(!text.contains("START"), "{text}");
    }

    #[test]
    fn a_dropped_action_is_marked_not_sent() {
        let (actions, now) = actions(0);
        let text = shown(&actions, Width::Wide, now);
        assert!(text.contains("Not sent: no timer connected"), "{text}");
    }
}
