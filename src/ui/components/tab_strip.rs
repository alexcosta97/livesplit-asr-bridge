//! Tab strip: the tabs of the tab area, with the active one underlined in
//! orange.

use eframe::egui::{
    Rect, RichText, Sense, Stroke, TextStyle, TextWrapMode, Ui, WidgetText, pos2, vec2,
};

use crate::ui::theme;

/// The tabs of the tab area. The Settings, Connection and Preferences tabs
/// join as they are built (#8, #10, #13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Log,
}

impl Tab {
    /// Every tab, in order.
    const ALL: [Self; 1] = [Self::Log];

    fn label(self) -> &'static str {
        match self {
            Self::Log => "Log",
        }
    }
}

/// The height of the strip, down to its bottom border.
const HEIGHT: f32 = 36.0;
/// The space between tab labels.
const GAP: f32 = 24.0;

/// The tab strip: each tab's label, the active one in primary text and
/// underlined in orange, over a 1 px border. Clicking a tab makes it active.
pub fn tab_strip(ui: &mut Ui, active: &mut Tab) {
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.painter().hline(
        strip.x_range(),
        strip.bottom() - 0.5,
        Stroke::new(1.0, theme::BORDER),
    );
    let mut x = strip.left();
    for tab in Tab::ALL {
        let text = RichText::new(tab.label().to_uppercase())
            .font(theme::body_semibold(12.0))
            .extra_letter_spacing(0.06 * 12.0);
        let galley = WidgetText::from(text).into_galley(
            ui,
            Some(TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Button,
        );
        let rect = Rect::from_min_size(pos2(x, strip.top()), vec2(galley.size().x, HEIGHT));
        let response = ui.interact(rect, ui.id().with(tab.label()), Sense::click());
        if response.clicked() {
            *active = tab;
        }
        let color = if tab == *active {
            theme::TEXT
        } else if response.hovered() {
            theme::TEXT_SECONDARY
        } else {
            theme::TEXT_MUTED
        };
        ui.painter().galley_with_override_text_color(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
        if tab == *active {
            ui.painter().hline(
                rect.x_range(),
                strip.bottom() - 1.0,
                Stroke::new(2.0, theme::ACCENT),
            );
        }
        x += rect.width() + GAP;
    }
}
