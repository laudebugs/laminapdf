//! Whether crash reports may be sent (D7a): off until the user says otherwise, asked once per
//! install after the first crash (S08), and changed only by that answer or by Settings › Privacy.
//! Kept in `crash.json` in the app's config folder.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};

use crate::store::{read_json, write_json};

/// The user's consent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub(crate) enum ConsentState {
    /// No crash has been asked about yet: nothing is sent, and the next crash asks.
    #[default]
    NotAsked,
    /// The user answered once: `send` says whether that report went. Nothing more is sent or asked.
    Asked { send: bool },
    /// Every report is sent, without asking.
    Always,
}

impl ConsentState {
    /// The state after the user answers the question: send this report or not, and always.
    pub(crate) const fn answered(send: bool, always: bool) -> Self {
        if send && always {
            Self::Always
        } else {
            Self::Asked { send }
        }
    }

    /// The state after Settings › Privacy's "Send crash reports" is switched.
    pub(crate) const fn switched(on: bool) -> Self {
        if on { Self::Always } else { Self::Asked { send: false } }
    }

    /// Whether a new report goes without asking.
    pub(crate) const fn sends(self) -> bool {
        matches!(self, Self::Always)
    }

    /// Whether the next crash asks.
    pub(crate) const fn asks(self) -> bool {
        matches!(self, Self::NotAsked)
    }
}

/// `crash.json`'s contents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Saved {
    consent: ConsentState,
}

/// The consent, in memory and in `crash.json`.
#[derive(Debug)]
pub(crate) struct ConsentStore {
    path: PathBuf,
    state: Mutex<ConsentState>,
}

impl ConsentStore {
    /// The consent saved at `path`; not asked when there is none.
    pub(crate) fn load(path: PathBuf) -> Self {
        let saved: Saved = read_json(&path, "crash report settings").unwrap_or_default();
        Self {
            path,
            state: Mutex::new(saved.consent),
        }
    }

    pub(crate) fn get(&self) -> ConsentState {
        *self.lock()
    }

    /// The consent without waiting: `None` while another thread changes it. For the panic hook,
    /// which mustn't block.
    pub(crate) fn try_get(&self) -> Option<ConsentState> {
        self.state.try_lock().ok().map(|state| *state)
    }

    /// Sets and saves the consent; a failed save keeps it for this run and is logged.
    pub(crate) fn set(&self, consent: ConsentState) {
        *self.lock() = consent;
        if let Err(error) = write_json(&self.path, &Saved { consent }) {
            tracing::warn!("could not save the crash report settings: {error}");
        }
    }

    fn lock(&self) -> MutexGuard<'_, ConsentState> {
        // One value: nothing a panic could leave half-done.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_install_should_send_nothing_and_ask_once() {
        let consent = ConsentState::default();
        assert_eq!((consent.sends(), consent.asks()), (false, true));
    }

    #[test]
    fn send_with_always_should_send_from_then_on() {
        assert_eq!(ConsentState::answered(true, true), ConsentState::Always);
    }

    #[test]
    fn send_without_always_should_neither_send_nor_ask_again() {
        let consent = ConsentState::answered(true, false);
        assert_eq!((consent.sends(), consent.asks()), (false, false));
    }

    #[test]
    fn dont_send_should_never_ask_again() {
        assert!(!ConsentState::answered(false, true).asks());
    }

    #[test]
    fn the_settings_switch_should_turn_sending_on_and_off() {
        assert_eq!(
            (
                ConsentState::switched(true).sends(),
                ConsentState::switched(false).sends()
            ),
            (true, false)
        );
    }

    #[test]
    fn the_consent_should_survive_a_restart() {
        let dir = std::env::temp_dir().join(format!("laminapdf-consent-{}", std::process::id()));
        let path = dir.join("crash.json");
        ConsentStore::load(path.clone()).set(ConsentState::Always);
        let again = ConsentStore::load(path).get();
        let _ = std::fs::remove_dir_all(dir);
        assert_eq!(again, ConsentState::Always);
    }
}
