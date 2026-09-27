//! The visual style: colours, fonts and egui visuals, from the brand tokens
//! in `assets/brand/tokens.json` (see `.superdesign/design-system.md`).

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Shadow, Stroke,
    TextStyle, Theme, Visuals,
};

/// The window background.
pub const WINDOW: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0C);
/// Panels, like the status column.
pub const PANEL: Color32 = Color32::from_rgb(0x15, 0x15, 0x17);
/// Raised surfaces and hover.
pub const RAISED: Color32 = Color32::from_rgb(0x1E, 0x1E, 0x21);
/// 1 px borders.
pub const BORDER: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2E);
/// Borders that stand out, like a secondary button's.
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x40);

/// Primary text.
pub const TEXT: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF4);
/// Secondary text, like help and details.
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);
/// Muted text, like labels and empty states.
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x6B, 0x6B, 0x73);

/// The one accent colour: primary buttons, focus, selection and links.
pub const ACCENT: Color32 = Color32::from_rgb(0xFF, 0x4D, 0x00);
/// The accent, hovered.
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xFF, 0x6A, 0x26);

// Status colours are for status only, never decoration.
/// Attached, connected.
pub const STATUS_OK: Color32 = Color32::from_rgb(0x22, 0xE0, 0x7A);
/// Waiting, not connected: normal states, not errors.
pub const STATUS_WAITING: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);
/// Errors.
pub const STATUS_ERROR: Color32 = Color32::from_rgb(0xFF, 0x3B, 0x3B);
/// Unsaved changes.
pub const STATUS_WARNING: Color32 = Color32::from_rgb(0xFF, 0xC2, 0x33);

/// The Connection category's tag in the log.
pub const LOG_CONNECTION: Color32 = Color32::from_rgb(0x7D, 0xD3, 0xFC);

/// A card's background in the OK state: a faint tint of green.
pub const TINT_OK: Color32 = Color32::from_rgb(0x0C, 0x18, 0x13);
/// The error card's background: a faint tint of red.
pub const TINT_ERROR: Color32 = Color32::from_rgb(0x1A, 0x0E, 0x0F);

/// The corner radius of cards, buttons and inputs.
pub const RADIUS: u8 = 4;

/// Archivo Black, for status words and titles, always uppercase.
const DISPLAY: &str = "display";
/// Inter SemiBold, for buttons and emphasis.
const BODY_SEMIBOLD: &str = "body-semibold";
/// Space Mono Bold.
const MONO_BOLD: &str = "mono-bold";

/// Archivo Black at `size`.
pub fn display(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY.into()))
}

/// Inter SemiBold at `size`.
pub fn body_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BODY_SEMIBOLD.into()))
}

/// Inter Regular at `size`.
pub fn body(size: f32) -> FontId {
    FontId::proportional(size)
}

/// Space Mono Regular at `size`.
pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Applies the fonts and the dark visuals. Called once, at startup.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    // The app has one, dark, look, whatever the system theme.
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| {
        style.visuals = visuals();
        style.text_styles = [
            (TextStyle::Small, body(11.0)),
            (TextStyle::Body, body(14.0)),
            (TextStyle::Button, body_semibold(13.0)),
            (TextStyle::Monospace, mono(12.0)),
            (TextStyle::Heading, display(20.0)),
        ]
        .into();
    });
}

fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let embedded: [(&str, &'static [u8]); 5] = [
        (
            "Inter-Regular",
            include_bytes!("../../assets/brand/fonts/Inter-Regular.ttf"),
        ),
        (
            "Inter-SemiBold",
            include_bytes!("../../assets/brand/fonts/Inter-SemiBold.ttf"),
        ),
        (
            "SpaceMono-Regular",
            include_bytes!("../../assets/brand/fonts/SpaceMono-Regular.ttf"),
        ),
        (
            "SpaceMono-Bold",
            include_bytes!("../../assets/brand/fonts/SpaceMono-Bold.ttf"),
        ),
        (
            "ArchivoBlack-Regular",
            include_bytes!("../../assets/brand/fonts/ArchivoBlack-Regular.ttf"),
        ),
    ];
    for (name, data) in embedded {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(data)));
    }

    // egui's own fonts stay as fallbacks, for symbols and emoji the brand
    // fonts don't have.
    let fallbacks = fonts.families[&FontFamily::Proportional].clone();
    let with_fallbacks = |font: &str| {
        std::iter::once(font.to_owned())
            .chain(fallbacks.iter().cloned())
            .collect::<Vec<_>>()
    };
    let families = [
        (FontFamily::Proportional, "Inter-Regular"),
        (FontFamily::Monospace, "SpaceMono-Regular"),
        (FontFamily::Name(DISPLAY.into()), "ArchivoBlack-Regular"),
        (FontFamily::Name(BODY_SEMIBOLD.into()), "Inter-SemiBold"),
        (FontFamily::Name(MONO_BOLD.into()), "SpaceMono-Bold"),
    ];
    for (family, font) in families {
        fonts.families.insert(family, with_fallbacks(font));
    }
    fonts
}

fn visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    let radius = CornerRadius::same(RADIUS);
    let border = Stroke::new(1.0, BORDER);

    visuals.panel_fill = WINDOW;
    visuals.window_fill = PANEL;
    visuals.window_stroke = Stroke::new(1.0, BORDER_STRONG);
    visuals.window_corner_radius = radius;
    visuals.menu_corner_radius = radius;
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.faint_bg_color = PANEL;
    visuals.extreme_bg_color = WINDOW;
    visuals.code_bg_color = RAISED;
    visuals.hyperlink_color = ACCENT;
    visuals.error_fg_color = STATUS_ERROR;
    visuals.warn_fg_color = STATUS_WARNING;
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.45);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.text_cursor.stroke = Stroke::new(2.0, ACCENT);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = PANEL;
    widgets.noninteractive.weak_bg_fill = PANEL;
    widgets.noninteractive.bg_stroke = border;
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.inactive.bg_fill = RAISED;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    widgets.hovered.bg_fill = RAISED;
    widgets.hovered.weak_bg_fill = RAISED;
    widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
    widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
    // Also the focused state, so focus is orange.
    widgets.active.bg_fill = RAISED;
    widgets.active.weak_bg_fill = RAISED;
    widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    widgets.active.fg_stroke = Stroke::new(1.5, TEXT);
    widgets.open = widgets.active;
    for state in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.corner_radius = radius;
        state.expansion = 0.0;
    }
    visuals
}
