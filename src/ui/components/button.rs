//! Button: primary or secondary, in three sizes.

use eframe::egui::{
    Color32, Response, Sense, Stroke, StrokeKind, Ui, Widget, WidgetInfo, WidgetType, vec2,
};

use crate::ui::theme;

/// The two looks of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Orange fill, for the main action.
    Primary,
    /// Outlined.
    Secondary,
}

/// The heights of a [`Button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// 32 px.
    Md,
    /// 28 px.
    Sm,
    /// 24 px.
    Xs,
}

impl ButtonSize {
    /// The height, horizontal padding and font size.
    fn metrics(self) -> (f32, f32, f32) {
        match self {
            Self::Md => (32.0, 14.0, 13.0),
            Self::Sm => (28.0, 10.0, 12.0),
            Self::Xs => (24.0, 8.0, 11.0),
        }
    }
}

/// A button, primary or secondary, in one of three sizes. A disabled button
/// is drawn at 40 % opacity and can't be clicked or focused.
#[must_use = "add it with `ui.add`"]
pub struct Button<'a> {
    label: &'a str,
    kind: ButtonKind,
    size: ButtonSize,
    enabled: bool,
    /// Replaces the label's colour, for a confirmation like "COPIED ✓".
    text_color: Option<Color32>,
}

impl<'a> Button<'a> {
    pub fn primary(label: &'a str) -> Self {
        Self::new(label, ButtonKind::Primary)
    }

    pub fn secondary(label: &'a str) -> Self {
        Self::new(label, ButtonKind::Secondary)
    }

    fn new(label: &'a str, kind: ButtonKind) -> Self {
        Self {
            label,
            kind,
            size: ButtonSize::Md,
            enabled: true,
            text_color: None,
        }
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn text_color(mut self, color: Color32) -> Self {
        self.text_color = Some(color);
        self
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let (height, padding, font_size) = self.size.metrics();
        let galley = ui.painter().layout_no_wrap(
            self.label.to_owned(),
            theme::body_semibold(font_size),
            Color32::PLACEHOLDER,
        );
        let size = vec2(galley.size().x + 2.0 * padding, height);
        let sense = if self.enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(size, sense);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, self.enabled, self.label));

        if ui.is_rect_visible(rect) {
            let hovered = self.enabled && response.hovered();
            let (fill, border, text) = match self.kind {
                ButtonKind::Primary => {
                    let fill = if hovered {
                        theme::ACCENT_HOVER
                    } else {
                        theme::ACCENT
                    };
                    (fill, fill, theme::WINDOW)
                }
                ButtonKind::Secondary => {
                    let fill = if hovered {
                        theme::RAISED
                    } else {
                        Color32::TRANSPARENT
                    };
                    (fill, theme::BORDER_STRONG, theme::TEXT)
                }
            };
            let text = self.text_color.unwrap_or(text);
            let opacity = if self.enabled { 1.0 } else { 0.4 };
            let painter = ui.painter();
            painter.rect(
                rect,
                theme::RADIUS,
                fill.gamma_multiply(opacity),
                Stroke::new(1.0, border.gamma_multiply(opacity)),
                StrokeKind::Inside,
            );
            if response.has_focus() {
                painter.rect_stroke(
                    rect.expand(2.0),
                    theme::RADIUS + 2,
                    Stroke::new(1.0, theme::ACCENT),
                    StrokeKind::Outside,
                );
            }
            let text_pos = rect.center() - galley.size() / 2.0;
            painter.galley(text_pos, galley, text.gamma_multiply(opacity));
        }
        response
    }
}
