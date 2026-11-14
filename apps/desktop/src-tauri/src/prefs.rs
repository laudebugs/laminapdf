//! The desktop app's own preferences, kept in `preferences.json` in the app's config folder:
//! whether it remembers recent files and whether it reloads files that change on disk. The
//! settings page (D5b, D7b) will show them; until then the Recent popover holds the toggles.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

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

    /// Replaces them and saves them.
    pub(crate) fn set(&self, value: Preferences) {
        *self.lock() = value;
        if let Err(error) = write_json(&self.file, &value) {
            tracing::warn!("could not save the preferences: {error}");
        }
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
            ..Preferences::default()
        };
        Prefs::load(file.clone()).set(off);
        let loaded = Prefs::load(file.clone()).get();
        let _ = std::fs::remove_dir_all(file.parent().unwrap_or(&file));
        assert_eq!(loaded, off);
    }
}
