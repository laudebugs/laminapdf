//! The user's update settings (ADR 0014 §1, §3), kept in `updates.json` in the app's config
//! folder: whether updates are automatic, the channel the user chose, when the last check ran, and
//! the anonymous install ID. The webview reads them through `update_status` and changes them only
//! through the update commands; it never sees or sets the install ID.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::store::{read_json, write_json};

/// A channel an install follows. `staging` is the tests' (D5c) and never one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Channel {
    /// Released versions. None exists before the first stable release.
    Stable,
    /// Pre-releases; every download is a beta until the first stable release.
    Beta,
    /// Nightly builds: a separate install line, signed with its own key (ADR 0014 §7).
    Nightly,
}

impl Channel {
    /// The channel's name in the update endpoint's path.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }

    /// The channel `name` names, in any case; `None` for any other name (`staging` included).
    pub(crate) fn parse(name: &str) -> Option<Self> {
        [Self::Stable, Self::Beta, Self::Nightly]
            .into_iter()
            .find(|channel| channel.as_str().eq_ignore_ascii_case(name.trim()))
    }
}

/// The settings as saved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct UpdateSettings {
    /// Check, download and install automatically. Off, the app makes no update request unless the
    /// user presses Check now.
    pub enabled: bool,
    /// The channel the user chose; `None` follows the build's own.
    pub channel: Option<Channel>,
    /// When the last check ran, in seconds since the Unix epoch.
    pub last_check: Option<u64>,
    /// The anonymous install ID, created on first use; never read by the webview.
    pub install_id: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            channel: None,
            last_check: None,
            install_id: None,
        }
    }
}

/// Whether `id` is a UUID v4, the only form the update Worker accepts.
fn is_install_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|id| id.get_version_num() == 4)
}

/// The settings and where they are saved.
pub(crate) struct UpdateStore {
    file: Option<PathBuf>,
    value: Mutex<UpdateSettings>,
}

impl UpdateStore {
    /// The settings saved at `file`, or the defaults.
    pub(crate) fn load(file: PathBuf) -> Self {
        let value = read_json(&file, "update settings").unwrap_or_default();
        Self {
            file: Some(file),
            value: Mutex::new(value),
        }
    }

    /// Settings that are never saved: the tests'.
    #[cfg(test)]
    pub(crate) fn in_memory(value: UpdateSettings) -> Self {
        Self {
            file: None,
            value: Mutex::new(value),
        }
    }

    pub(crate) fn get(&self) -> UpdateSettings {
        self.lock().clone()
    }

    /// Changes the settings with `change` and saves them; returns what `change` returned.
    pub(crate) fn update<T>(&self, change: impl FnOnce(&mut UpdateSettings) -> T) -> T {
        let mut value = self.lock();
        let answer = change(&mut value);
        if let Some(file) = &self.file
            && let Err(error) = write_json(file, &*value)
        {
            tracing::warn!("could not save the update settings: {error}");
        }
        answer
    }

    /// The install ID, created (and saved) on first use, or anew when the saved one isn't a UUID v4.
    pub(crate) fn install_id(&self) -> String {
        if let Some(id) = self.lock().install_id.clone().filter(|id| is_install_id(id)) {
            return id;
        }
        self.update(|settings| {
            let id = uuid::Uuid::new_v4().to_string();
            settings.install_id = Some(id.clone());
            id
        })
    }

    /// Replaces the install ID with a new one ("Reset anonymous ID").
    pub(crate) fn reset_install_id(&self) {
        self.update(|settings| settings.install_id = Some(uuid::Uuid::new_v4().to_string()));
    }

    fn lock(&self) -> MutexGuard<'_, UpdateSettings> {
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_should_parse_in_any_case_and_never_as_staging() {
        assert_eq!(Channel::parse(" Beta "), Some(Channel::Beta));
        assert_eq!(Channel::parse("STABLE"), Some(Channel::Stable));
        assert_eq!(Channel::parse("staging"), None);
        assert_eq!(Channel::parse(""), None);
    }

    #[test]
    fn missing_keys_should_take_their_defaults() {
        let read: UpdateSettings = serde_json::from_str(r#"{ "channel": "beta" }"#).unwrap();
        assert_eq!(
            read,
            UpdateSettings {
                channel: Some(Channel::Beta),
                ..UpdateSettings::default()
            }
        );
    }

    #[test]
    fn the_install_id_should_be_made_once_and_kept() {
        let store = UpdateStore::in_memory(UpdateSettings::default());
        let id = store.install_id();
        assert!(is_install_id(&id), "{id}");
        assert_eq!(store.install_id(), id);
    }

    #[test]
    fn an_install_id_that_isnt_a_uuid_v4_should_be_replaced() {
        let store = UpdateStore::in_memory(UpdateSettings {
            install_id: Some("not-a-uuid".to_owned()),
            ..UpdateSettings::default()
        });
        assert!(is_install_id(&store.install_id()));
    }

    #[test]
    fn resetting_should_give_a_new_install_id() {
        let store = UpdateStore::in_memory(UpdateSettings::default());
        let before = store.install_id();
        store.reset_install_id();
        let after = store.install_id();
        assert!(is_install_id(&after) && after != before);
    }

    #[test]
    fn saved_settings_should_load_again() {
        let file = std::env::temp_dir().join(format!("laminapdf-updates-{}/updates.json", std::process::id()));
        let store = UpdateStore::load(file.clone());
        store.update(|settings| {
            settings.enabled = false;
            settings.channel = Some(Channel::Beta);
        });
        let id = store.install_id();
        let loaded = UpdateStore::load(file.clone()).get();
        let _ = std::fs::remove_dir_all(file.parent().unwrap_or(&file));
        assert_eq!(
            loaded,
            UpdateSettings {
                enabled: false,
                channel: Some(Channel::Beta),
                last_check: None,
                install_id: Some(id),
            }
        );
    }
}
