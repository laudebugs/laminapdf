//! Locate… on a restored tab whose file is missing: the user picks where the file went. If the pick
//! looks different from the file the session had (another size or date), it waits for the user's
//! answer before it opens, so a different document never takes the tab's place unasked.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use tauri::Manager as _;

use super::Stamp;
use super::app::{missing, open_restored};
use super::track::Session;
use crate::DesktopError;
use crate::engine::DocumentMeta;

/// Whether a file stamped `now` looks different from the one the session saved as `saved`: another
/// size, or another modification time where both are known.
pub(crate) fn looks_different(saved: Stamp, now: Stamp) -> bool {
    saved.len != now.len || matches!((saved.mtime, now.mtime), (Some(before), Some(after)) if before != after)
}

/// Which restored tab a held pick is for: its window and its token.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HeldFor {
    window: String,
    token: String,
}

/// Picks that looked different, by the tab they are for, until its window answers or closes.
#[derive(Debug, Default)]
pub(crate) struct Held(Mutex<BTreeMap<HeldFor, PathBuf>>);

impl Held {
    fn picks(&self) -> MutexGuard<'_, BTreeMap<HeldFor, PathBuf>> {
        // A map of paths: nothing a panic could leave half-done.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn hold(&self, window: &str, token: &str, path: PathBuf) {
        self.picks().insert(held_for(window, token), path);
    }

    fn take(&self, window: &str, token: &str) -> Option<PathBuf> {
        self.picks().remove(&held_for(window, token))
    }

    /// `window` closed: what it held goes, unanswered.
    pub(crate) fn closed(&self, window: &str) {
        self.picks().retain(|held, _| held.window != window);
    }
}

fn held_for(window: &str, token: &str) -> HeldFor {
    HeldFor {
        window: window.to_owned(),
        token: token.to_owned(),
    }
}

/// What Locate… found.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum Located {
    /// The pick looked like the session's file and opened.
    Opened { document: DocumentMeta },
    /// The pick looks different: it opens only after `session_locate_answer` says so.
    Different { name: String },
}

/// Lets the user pick where restored tab `token`'s file went. A pick that looks like the file the
/// session had opens in this window; one that looks different is held for
/// [`session_locate_answer`]. `null` when the user cancels.
///
/// # Errors
/// [`DesktopError::Io`] for an unknown token or a pick that can't be read, and whatever opening
/// the file reports.
#[tauri::command]
pub(crate) async fn session_locate(
    window: tauri::WebviewWindow,
    token: String,
) -> Result<Option<Located>, DesktopError> {
    // The dialog and the engine's replies block: off the async runtime's threads.
    tauri::async_runtime::spawn_blocking(move || {
        let saved = window
            .state::<std::sync::Arc<Session>>()
            .tracker()
            .restored_tab(&token, window.label())
            .map(|tab| tab.stamp)
            .ok_or_else(missing)?;
        let Some(path) = crate::commands::pick_pdf(&window)? else {
            return Ok(None);
        };
        if !looks_different(saved, Stamp::of(&path)?) {
            return open_restored(&window, &token, path).map(|document| Some(Located::Opened { document }));
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        // A window that closed while its picker was up holds nothing: Held::closed has run.
        if window.app_handle().get_webview_window(window.label()).is_none() {
            return Ok(None);
        }
        window.state::<Held>().hold(window.label(), &token, path);
        Ok(Some(Located::Different { name }))
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// The answer to a pick [`session_locate`] held: `open` opens it as restored tab `token`'s file,
/// else it is dropped. Returns the opened document, or `null` when it was dropped.
///
/// # Errors
/// [`DesktopError::Io`] when no pick is held for `token` in this window, and whatever opening the
/// file reports.
#[tauri::command]
pub(crate) async fn session_locate_answer(
    window: tauri::WebviewWindow,
    token: String,
    open: bool,
) -> Result<Option<DocumentMeta>, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = window
            .state::<Held>()
            .take(window.label(), &token)
            .ok_or_else(missing)?;
        // The tab may have closed, or its file been found, while the question was open.
        let still_missing = window
            .state::<std::sync::Arc<Session>>()
            .tracker()
            .restored_tab(&token, window.label())
            .is_some();
        if !open || !still_missing {
            return Ok(None);
        }
        open_restored(&window, &token, path).map(Some)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(len: u64, mtime: Option<u64>) -> Stamp {
        Stamp { len, mtime }
    }

    #[test]
    fn a_closed_window_should_drop_only_its_own_picks() {
        let held = Held::default();
        held.hold("main", "t1", PathBuf::from("a.pdf"));
        held.hold("doc-1", "t2", PathBuf::from("b.pdf"));
        held.closed("main");
        assert_eq!(
            (held.take("main", "t1"), held.take("doc-1", "t2")),
            (None, Some(PathBuf::from("b.pdf")))
        );
    }

    #[test]
    fn the_same_size_and_date_should_look_the_same() {
        assert!(!looks_different(stamp(10, Some(5)), stamp(10, Some(5))));
    }

    #[test]
    fn another_size_should_look_different() {
        assert!(looks_different(stamp(10, Some(5)), stamp(11, Some(5))));
    }

    #[test]
    fn another_date_should_look_different() {
        assert!(looks_different(stamp(10, Some(5)), stamp(10, Some(6))));
    }

    #[test]
    fn an_unknown_date_should_leave_the_size_to_decide() {
        assert!(!looks_different(stamp(10, None), stamp(10, Some(6))));
        assert!(looks_different(stamp(10, Some(5)), stamp(12, None)));
    }
}
