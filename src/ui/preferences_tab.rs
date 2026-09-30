//! The Preferences tab (spec §6.7): whether the window's size and position
//! are remembered, developer mode, and the About section.

use std::path::Path;

use eframe::egui::{RichText, ScrollArea, Ui};

use super::components::{self, Checkbox};
use super::{theme, window_title};

/// The space between the tab's sections.
const SECTION_GAP: f32 = 48.0;
/// The space under a section label.
const LABEL_GAP: f32 = 16.0;
/// The space between the lines of the About section.
const ROW_GAP: f32 = 16.0;

/// What the Preferences tab asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesAction {
    /// Remember window size and position was ticked or unticked.
    RememberWindowChanged,
    /// Developer mode was ticked or unticked.
    DeveloperModeChanged,
    OpenConfigFolder,
    OpenLogFolder,
}

/// Shows the Preferences tab, with the preferences in `remember_window` and
/// `developer_mode` and the folders to show in About. Returns what was
/// clicked.
pub fn preferences_tab(
    ui: &mut Ui,
    remember_window: &mut bool,
    developer_mode: &mut bool,
    config_dir: Option<&Path>,
    log_dir: Option<&Path>,
) -> Option<PreferencesAction> {
    let mut action = None;
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        components::section_label(ui, "Window");
        ui.add_space(LABEL_GAP);
        let checkbox = Checkbox::new(remember_window, "Remember window size and position").note(
            "Turn this off with a tiling window manager, so the window manager decides the size.",
        );
        if ui.add(checkbox).changed() {
            action = Some(PreferencesAction::RememberWindowChanged);
        }

        ui.add_space(SECTION_GAP);
        components::section_label(ui, "Auto splitter development");
        ui.add_space(LABEL_GAP);
        let checkbox = Checkbox::new(developer_mode, "Developer mode").note(
            "Shows the auto splitter's settings map in Settings, and its messages in Log. \
             For writing or debugging auto splitters.",
        );
        if ui.add(checkbox).changed() {
            action = Some(PreferencesAction::DeveloperModeChanged);
        }

        ui.add_space(SECTION_GAP);
        components::section_label(ui, "About");
        ui.add_space(LABEL_GAP);
        ui.label(
            RichText::new(window_title())
                .font(theme::mono(13.0))
                .color(theme::TEXT),
        );
        ui.add_space(ROW_GAP);
        if components::about_folder(ui, "Config folder", config_dir) {
            action = Some(PreferencesAction::OpenConfigFolder);
        }
        ui.add_space(ROW_GAP);
        if components::about_folder(ui, "Log folder", log_dir) {
            action = Some(PreferencesAction::OpenLogFolder);
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use eframe::egui::{
        Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, accesskit, pos2, vec2,
    };

    use super::*;

    /// A widget on screen, as the accessibility tree describes it.
    #[derive(Debug)]
    struct Node {
        label: String,
        rect: Rect,
        disabled: bool,
        toggled: Option<bool>,
    }

    /// The Preferences tab drawn without a window, with the app's fonts.
    struct Screen {
        ctx: Context,
        config_dir: Option<PathBuf>,
        log_dir: Option<PathBuf>,
        /// The developer mode preference, as the tab leaves it.
        developer: std::cell::Cell<bool>,
    }

    impl Screen {
        fn new(config_dir: Option<&str>, log_dir: Option<&str>) -> Self {
            let ctx = Context::default();
            crate::ui::theme::install(&ctx);
            ctx.enable_accesskit();
            Self {
                ctx,
                config_dir: config_dir.map(PathBuf::from),
                log_dir: log_dir.map(PathBuf::from),
                developer: std::cell::Cell::new(false),
            }
        }

        /// Draws one frame with `events`, returning the tab's action and
        /// what is on screen.
        fn frame(
            &self,
            remember: &mut bool,
            events: Vec<Event>,
        ) -> (Option<PreferencesAction>, Vec<Node>) {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(700.0, 800.0))),
                events,
                ..RawInput::default()
            };
            let mut action = None;
            let mut developer = self.developer.get();
            let mut output = self.ctx.run_ui(input, |ui| {
                action = preferences_tab(
                    ui,
                    remember,
                    &mut developer,
                    self.config_dir.as_deref(),
                    self.log_dir.as_deref(),
                );
            });
            self.developer.set(developer);
            // Nothing draws the frame, so its textures are dropped.
            output.textures_delta.clear();
            let nodes = output
                .platform_output
                .accesskit_update
                .map(|update| update.nodes)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(_, node): (accesskit::NodeId, accesskit::Node)| {
                    let bounds = node.bounds()?;
                    Some(Node {
                        label: node.label().or(node.value())?.to_owned(),
                        rect: Rect::from_min_max(
                            pos2(bounds.x0 as f32, bounds.y0 as f32),
                            pos2(bounds.x1 as f32, bounds.y1 as f32),
                        ),
                        disabled: node.is_disabled(),
                        toggled: node
                            .toggled()
                            .map(|toggled| toggled == accesskit::Toggled::True),
                    })
                })
                .collect();
            (action, nodes)
        }

        /// Clicks the `index`th widget labelled `label`, returning the tab's
        /// action.
        fn click(
            &self,
            remember: &mut bool,
            label: &str,
            index: usize,
        ) -> Option<PreferencesAction> {
            let (_, nodes) = self.frame(remember, Vec::new());
            let position = find_all(&nodes, label)[index].rect.center();
            let button = |pressed| Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            let (pressed, _) =
                self.frame(remember, vec![Event::PointerMoved(position), button(true)]);
            let (released, _) = self.frame(remember, vec![button(false)]);
            pressed.or(released)
        }
    }

    fn find_all<'a>(nodes: &'a [Node], label: &str) -> Vec<&'a Node> {
        let mut found: Vec<_> = nodes.iter().filter(|node| node.label == label).collect();
        // Top to bottom, as they are on screen.
        found.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
        assert!(!found.is_empty(), "no {label:?} in {nodes:#?}");
        found
    }

    const REMEMBER: &str = "Remember window size and position";

    #[test]
    fn the_preference_shows_ticked_and_unticks_when_clicked() {
        let screen = Screen::new(Some("/config"), Some("/logs"));
        let mut remember = true;
        let (_, nodes) = screen.frame(&mut remember, Vec::new());
        assert_eq!(find_all(&nodes, REMEMBER)[0].toggled, Some(true));
        assert_eq!(
            screen.click(&mut remember, REMEMBER, 0),
            Some(PreferencesAction::RememberWindowChanged)
        );
        assert!(!remember);
    }

    const DEVELOPER: &str = "Developer mode";

    #[test]
    fn developer_mode_shows_unticked_and_ticks_when_clicked() {
        let screen = Screen::new(Some("/config"), Some("/logs"));
        let (_, nodes) = screen.frame(&mut true, Vec::new());
        find_all(&nodes, "AUTO SPLITTER DEVELOPMENT");
        assert_eq!(find_all(&nodes, DEVELOPER)[0].toggled, Some(false));
        assert_eq!(
            screen.click(&mut true, DEVELOPER, 0),
            Some(PreferencesAction::DeveloperModeChanged)
        );
        assert!(screen.developer.get());
        // Nothing marks it as new: the mockup's tag was a note, not UI.
        let (_, nodes) = screen.frame(&mut true, Vec::new());
        assert!(
            !nodes.iter().any(|node| node.label.contains("NEW")),
            "{nodes:#?}"
        );
    }

    #[test]
    fn about_shows_the_name_and_version_and_both_folders() {
        let screen = Screen::new(Some("/config"), Some("/logs"));
        let (_, nodes) = screen.frame(&mut true, Vec::new());
        let version = format!("LiveSplit One ASR Bridge {}", crate::version::VERSION);
        for label in [
            "WINDOW",
            "ABOUT",
            &version,
            "Config folder",
            "/config",
            "Log folder",
            "/logs",
        ] {
            find_all(&nodes, label);
        }
    }

    #[test]
    fn open_opens_the_folder_of_its_row() {
        let screen = Screen::new(Some("/config"), Some("/logs"));
        assert_eq!(
            screen.click(&mut true, "Open", 0),
            Some(PreferencesAction::OpenConfigFolder)
        );
        assert_eq!(
            screen.click(&mut true, "Open", 1),
            Some(PreferencesAction::OpenLogFolder)
        );
    }

    #[test]
    fn a_folder_that_is_not_found_cannot_be_opened() {
        let screen = Screen::new(None, Some("/logs"));
        let (_, nodes) = screen.frame(&mut true, Vec::new());
        find_all(&nodes, "Not found");
        let open = find_all(&nodes, "Open");
        assert!(open[0].disabled);
        assert!(!open[1].disabled);
    }

    #[test]
    fn a_long_path_fits_the_row() {
        let long = format!("/home/{}/config", "a".repeat(200));
        let screen = Screen::new(Some(&long), Some("/logs"));
        let (_, nodes) = screen.frame(&mut true, Vec::new());
        let open = find_all(&nodes, "Open")[0];
        assert!(open.rect.right() <= 700.0, "{open:?}");
        let path = find_all(&nodes, &long)[0];
        assert!(path.rect.right() <= open.rect.left(), "{path:?} {open:?}");
    }
}
