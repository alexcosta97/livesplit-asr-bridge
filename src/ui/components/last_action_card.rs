//! Last action card: the auto splitter's latest timer action, and the 2
//! before it.

use std::time::{Duration, Instant};

use eframe::egui::{Align, Label, Layout, RichText, Ui};

use super::{
    Width,
    card::{Edge, card},
    section_label::section_label,
    status_word::{display_text, status_word},
};
use crate::ui::{
    last_action::{LastActions, ShownAction},
    theme,
};

/// How long the card's background flashes when a new action arrives.
const FLASH: Duration = Duration::from_millis(300);

/// The Last action card: the label with the latest action's time, the action
/// in large type, a split's segment name when the timer gave it, a note when
/// it wasn't sent, and the 2 previous actions as faint lines; or the empty
/// state. While it shows an action it has an orange edge, and its background
/// flashes orange as an action arrives.
pub fn last_action_card(ui: &mut Ui, actions: &LastActions, width: Width, now: Instant) {
    let latest = actions.latest();
    let flashing = latest
        .map(|latest| FLASH.saturating_sub(now.saturating_duration_since(latest.arrived)))
        .filter(|left| !left.is_zero());
    if let Some(left) = flashing {
        ui.ctx().request_repaint_after(left);
    }
    let edge = latest.map(|_| Edge {
        color: theme::ACCENT,
        tint: if flashing.is_some() {
            theme::TINT_ACCENT
        } else {
            theme::WINDOW
        },
    });
    card(ui, edge, |ui| {
        ui.horizontal(|ui| {
            section_label(ui, "Last action");
            if let Some(latest) = latest {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(time(latest));
                });
            }
        });
        ui.add_space(8.0);
        let Some(latest) = latest else {
            status_word(ui, None, "No actions yet", theme::TEXT_MUTED, width);
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Actions appear here when the auto splitter starts, splits or resets.",
                )
                .font(theme::body(13.0))
                .color(theme::TEXT_SECONDARY),
            );
            return;
        };
        let size = match width {
            Width::Wide => 24.0,
            Width::Compact => 30.0,
        };
        ui.add(
            Label::new(
                display_text(&latest.word, size, theme::ACCENT)
                    .line_height(Some((size * 1.05).round())),
            )
            .wrap(),
        );
        // Only when the timer gave it: the app never makes one up.
        if let Some(segment) = &latest.segment {
            ui.add_space(4.0);
            let size = match width {
                Width::Wide => 14.0,
                Width::Compact => 16.0,
            };
            let name = RichText::new(segment)
                .font(theme::body_semibold(size))
                .color(theme::TEXT);
            ui.add(Label::new(name).truncate());
        }
        if !latest.sent {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Not sent: no timer connected")
                    .font(theme::mono(12.0))
                    .color(theme::STATUS_WARNING),
            );
        }
        for (index, action) in actions.previous().enumerate() {
            ui.add_space(if index == 0 { 12.0 } else { 4.0 });
            let mut line = format!(
                "{}  {}",
                action.time.format("%H:%M:%S"),
                action.word.to_uppercase()
            );
            if let Some(segment) = &action.segment {
                line.push_str(&format!(" · {segment}"));
            }
            ui.add(
                Label::new(
                    RichText::new(line)
                        .font(theme::mono(12.0))
                        .color(theme::TEXT_MUTED),
                )
                .truncate(),
            );
        }
    });
}

/// An action's time of day, like `19:31:01`.
fn time(action: &ShownAction) -> RichText {
    RichText::new(action.time.format("%H:%M:%S").to_string())
        .font(theme::mono(12.0))
        .color(theme::TEXT_MUTED)
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
            let event = RunnerEvent::TimerAction {
                action,
                sent_to,
                segment: None,
            };
            actions.apply(&event, time, now);
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
    fn previous_actions_read_time_first() {
        let (actions, now) = actions(1);
        let text = shown(&actions, Width::Wide, now);
        assert!(text.contains("19:31:01  START"), "{text}");
        assert!(text.contains("19:31:00  RESET"), "{text}");
    }

    #[test]
    fn compact_also_shows_the_two_before_it() {
        let (actions, now) = actions(1);
        let text = shown(&actions, Width::Compact, now);
        for expected in ["SPLIT", "19:31:02", "19:31:01  START", "19:31:00  RESET"] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }

    #[test]
    fn a_split_shows_its_segment_name() {
        let mut actions = LastActions::default();
        let now = Instant::now();
        let time = NaiveTime::from_hms_opt(19, 31, 1).unwrap();
        for segment in ["Los Santos — Ryder", "Los Santos — Gym Moves"] {
            let event = RunnerEvent::TimerAction {
                action: TimerAction::Split,
                sent_to: 1,
                segment: Some(segment.to_owned()),
            };
            actions.apply(&event, time, now);
        }
        let text = shown(&actions, Width::Wide, now);
        assert!(text.contains("Los Santos — Gym Moves"), "{text}");
        assert!(text.contains("SPLIT · Los Santos — Ryder"), "{text}");
        let text = shown(&actions, Width::Compact, now);
        assert!(text.contains("Los Santos — Gym Moves"), "{text}");
    }

    #[test]
    fn a_dropped_action_is_marked_not_sent() {
        let (actions, now) = actions(0);
        let text = shown(&actions, Width::Wide, now);
        assert!(text.contains("Not sent: no timer connected"), "{text}");
    }
}
