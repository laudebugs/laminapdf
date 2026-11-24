//! The desktop app's own preferences, kept in `preferences.json` in the app's config folder:
//! whether it remembers recent files and whether it reloads files that change on disk, where files
//! open, Settings › Appearance and accessibility (theme, accent, page appearance, motion) and, on
//! Linux, Settings › Graphics (`graphics.rs`, read before the app starts). Settings shows and
//! changes them all (`src/settings.tsx`).

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use tauri::Theme;

use crate::graphics::Graphics;
use crate::store::{read_json, write_json};

/// The preferences, as the webview reads and sets them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is one independent switch in Settings, not a state an enum would name"
)]
pub(crate) struct Preferences {
    /// Record opened files in the app's and the OS's recent lists. Off clears both.
    pub remember_recent: bool,
    /// Reload an open document when its file changes on disk.
    pub reload_on_change: bool,
    /// Where a file the OS hands the app opens: a new tab of the frontmost window, or a new window.
    pub open_in: OpenIn,
    /// Open a file that is open already again, in another tab, instead of showing its tab.
    pub allow_duplicates: bool,
    /// Bring back the last session's windows and tabs on launch; kept only while
    /// `remember_recent` is on too (D4d's privacy rule).
    pub reopen_last: bool,
    /// Settings › Appearance: the UI theme (`light`, `dark`, `high-contrast`, or `system`).
    pub theme: UiTheme,
    /// Settings › Appearance: the accent colour (`teal`, `blue`, `plum`, or `green`).
    pub accent: Accent,
    /// Settings › Appearance: how document pages are drawn (`None` before the user chooses, so
    /// `forced-colors: active` defaults to High contrast and otherwise Original).
    #[serde(deserialize_with = "deserialize_page_appearance")]
    pub page_appearance: Option<PageAppearance>,
    /// Settings › Motion: follow the system, or reduce motion whatever it says.
    pub motion: Motion,
    /// Settings › Appearance: whether single-key shortcuts are on (WCAG 2.1.4).
    pub single_keys: bool,
    /// Settings › Graphics (Linux): the `WebKitGTK` workarounds the next start runs with.
    pub graphics: Graphics,
}

/// Settings › Appearance: the UI theme (matches `@laminapdf/ui`'s `UiTheme`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum UiTheme {
    /// Paper's light palette.
    Light,
    /// Paper's dark palette.
    Dark,
    /// System high-contrast colours (`Canvas` / `CanvasText`).
    HighContrast,
    /// Follow the OS's light or dark theme; also any value a later version wrote, so the other
    /// preferences still load (serde's `other` variant, which must come last).
    #[default]
    #[serde(other)]
    System,
}

impl UiTheme {
    /// The native window theme to request (`None` lets the window follow the OS).
    pub(crate) const fn window_theme(self) -> Option<Theme> {
        match self {
            Self::Light => Some(Theme::Light),
            Self::Dark => Some(Theme::Dark),
            Self::HighContrast | Self::System => None,
        }
    }
}

/// Settings › Appearance: the accent colour (matches `@laminapdf/ui`'s `Accent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Accent {
    /// Blue accent.
    Blue,
    /// Plum accent.
    Plum,
    /// Green accent.
    Green,
    /// Teal accent (default); also any value a later version wrote.
    #[default]
    #[serde(other)]
    Teal,
}

/// Settings › Appearance: how document pages are drawn (matches `@laminapdf/tokens`'s `PageAppearance`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PageAppearance {
    /// The document's own colours.
    Original,
    /// Warm sepia paper.
    Sepia,
    /// Dark paper with inverted text.
    Dark,
    /// System high-contrast ink and paper.
    HighContrast,
}

/// Reads an optional [`PageAppearance`], turning unknown future values into `None` so the rest of
/// `preferences.json` still loads.
fn deserialize_page_appearance<'de, D>(deserializer: D) -> Result<Option<PageAppearance>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    enum Wire {
        Original,
        Sepia,
        Dark,
        HighContrast,
        #[serde(other)]
        Unknown,
    }
    Ok(match Option::<Wire>::deserialize(deserializer)? {
        Some(Wire::Original) => Some(PageAppearance::Original),
        Some(Wire::Sepia) => Some(PageAppearance::Sepia),
        Some(Wire::Dark) => Some(PageAppearance::Dark),
        Some(Wire::HighContrast) => Some(PageAppearance::HighContrast),
        Some(Wire::Unknown) | None => None,
    })
}

/// Settings › Motion (motion plan §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Motion {
    /// Nothing slides or grows; fades stay.
    Reduce,
    /// The system's setting, read by the webview and, where it misses it, by `motion.rs`; also
    /// any value a later version wrote, so the other preferences still load (serde's `other`
    /// variant, which must come last).
    #[default]
    #[serde(other)]
    System,
}

/// Where the files the OS hands the app open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum OpenIn {
    /// In a new tab of the frontmost window.
    #[default]
    Tabs,
    /// Each in a new window of its own.
    Windows,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            remember_recent: true,
            reload_on_change: true,
            open_in: OpenIn::Tabs,
            allow_duplicates: false,
            reopen_last: true,
            theme: UiTheme::System,
            accent: Accent::Teal,
            page_appearance: None,
            motion: Motion::System,
            single_keys: true,
            graphics: Graphics::Automatic,
        }
    }
}

impl Preferences {
    /// Whether going from these to `next` turns "Remember recent files" off, which clears the
    /// app's and the OS's recent lists.
    pub(crate) const fn stops_remembering(self, next: Self) -> bool {
        self.remember_recent && !next.remember_recent
    }
}

/// The preferences and where they are saved.
pub(crate) struct Prefs {
    file: PathBuf,
    value: Mutex<Preferences>,
}

impl Prefs {
    /// The preferences saved at `file`, or the defaults.
    pub(crate) fn load(file: PathBuf) -> Self {
        let value = read_json(&file, "preferences").unwrap_or_default();
        Self {
            file,
            value: Mutex::new(value),
        }
    }

    pub(crate) fn get(&self) -> Preferences {
        *self.lock()
    }

    /// Replaces the preferences atomically, writes them to disk while holding the lock, and
    /// returns the previous value.
    pub(crate) fn replace(&self, value: Preferences) -> Preferences {
        let mut guard = self.lock();
        let was = std::mem::replace(&mut *guard, value);
        if let Err(error) = write_json(&self.file, &value) {
            tracing::warn!("could not save the preferences: {error}");
        }
        was
    }

    fn lock(&self) -> MutexGuard<'_, Preferences> {
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_keys_should_take_their_defaults() {
        let read: Preferences = serde_json::from_str(r#"{ "reloadOnChange": false }"#).unwrap();
        assert_eq!(
            read,
            Preferences {
                remember_recent: true,
                reload_on_change: false,
                ..Preferences::default()
            }
        );
    }

    #[test]
    fn an_unknown_motion_should_read_as_the_system_and_keep_the_rest() {
        let read: Preferences = serde_json::from_str(r#"{ "rememberRecent": false, "motion": "full" }"#).unwrap();
        assert_eq!((read.motion, read.remember_recent), (Motion::System, false));
    }

    #[test]
    fn old_preferences_without_view_fields_should_load_with_defaults() {
        let old = r#"{
            "rememberRecent": true,
            "reloadOnChange": false,
            "openIn": "windows",
            "allowDuplicates": true,
            "reopenLast": false,
            "motion": "reduce",
            "graphics": "automatic"
        }"#;
        let read: Preferences = serde_json::from_str(old).unwrap();
        assert_eq!(
            (read.theme, read.accent, read.page_appearance, read.single_keys),
            (UiTheme::System, Accent::Teal, None, true)
        );
        assert_eq!(
            (read.reload_on_change, read.open_in, read.motion),
            (false, OpenIn::Windows, Motion::Reduce)
        );
    }

    #[test]
    fn unknown_view_choices_should_fall_back_to_defaults_and_keep_the_rest() {
        let read: Preferences = serde_json::from_str(
            r#"{ "rememberRecent": false, "theme": "solarized", "accent": "red", "pageAppearance": "neon" }"#,
        )
        .unwrap();
        assert_eq!(
            (read.remember_recent, read.theme, read.accent, read.page_appearance),
            (false, UiTheme::System, Accent::Teal, None)
        );
    }

    #[test]
    fn view_choices_should_serialise_in_kebab_case_under_camel_case_keys() {
        let prefs = Preferences {
            theme: UiTheme::HighContrast,
            accent: Accent::Plum,
            page_appearance: Some(PageAppearance::HighContrast),
            single_keys: false,
            ..Preferences::default()
        };
        let json = serde_json::to_value(prefs).unwrap();
        assert_eq!(json["theme"], "high-contrast");
        assert_eq!(json["accent"], "plum");
        assert_eq!(json["pageAppearance"], "high-contrast");
        assert_eq!(json["singleKeys"], false);
        assert_eq!(serde_json::from_value::<Preferences>(json).unwrap(), prefs);
    }

    #[test]
    fn view_choice_enums_should_match_menus_json() {
        fn ser<T: Serialize>(value: &T) -> String {
            serde_json::to_value(value).unwrap().as_str().unwrap().to_owned()
        }
        let menus: serde_json::Value = serde_json::from_str(include_str!("../menus.json")).unwrap();
        let choices_for = |command_id: &str| -> Vec<String> {
            menus["menus"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|menu| menu["items"].as_array().unwrap())
                .find(|item| item["id"].as_str() == Some(command_id))
                .and_then(|item| item["choices"].as_array())
                .unwrap()
                .iter()
                .map(|choice| choice["value"].as_str().unwrap().to_owned())
                .collect()
        };

        let themes: Vec<String> = [UiTheme::Light, UiTheme::Dark, UiTheme::HighContrast, UiTheme::System]
            .iter()
            .map(ser)
            .collect();
        let accents: Vec<String> = [Accent::Teal, Accent::Blue, Accent::Plum, Accent::Green]
            .iter()
            .map(ser)
            .collect();
        let pages: Vec<String> = [
            PageAppearance::Original,
            PageAppearance::Sepia,
            PageAppearance::Dark,
            PageAppearance::HighContrast,
        ]
        .iter()
        .map(ser)
        .collect();

        assert_eq!(themes, choices_for("view.theme"));
        assert_eq!(accents, choices_for("view.accent"));
        assert_eq!(pages, choices_for("view.page-appearance"));
    }

    #[test]
    fn window_theme_follows_os_for_system_and_high_contrast() {
        assert_eq!(UiTheme::Light.window_theme(), Some(Theme::Light));
        assert_eq!(UiTheme::Dark.window_theme(), Some(Theme::Dark));
        assert_eq!(UiTheme::System.window_theme(), None);
        assert_eq!(UiTheme::HighContrast.window_theme(), None);
    }

    #[test]
    fn turning_remember_off_should_clear_the_lists() {
        let on = Preferences::default();
        let off = Preferences {
            remember_recent: false,
            ..on
        };
        assert_eq!(
            [
                on.stops_remembering(off),
                off.stops_remembering(off),
                off.stops_remembering(on)
            ],
            [true, false, false]
        );
    }

    #[test]
    fn saved_preferences_should_load_again() {
        let file = std::env::temp_dir().join(format!("laminapdf-prefs-{}/preferences.json", std::process::id()));
        let off = Preferences {
            remember_recent: false,
            open_in: OpenIn::Windows,
            theme: UiTheme::Dark,
            accent: Accent::Green,
            page_appearance: Some(PageAppearance::Sepia),
            motion: Motion::Reduce,
            single_keys: false,
            ..Preferences::default()
        };
        let was = Prefs::load(file.clone()).replace(off);
        assert_eq!(was, Preferences::default());
        let loaded = Prefs::load(file.clone()).get();
        let _ = std::fs::remove_dir_all(file.parent().unwrap_or(&file));
        assert_eq!(loaded, off);
    }
}
