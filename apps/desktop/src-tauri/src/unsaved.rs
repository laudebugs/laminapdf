//! The close and quit guard (E1h): a window whose document has edits not yet saved says so, and
//! closing it, or quitting the app, asks it first. The webview shows the question; once the user
//! chose to discard the edits, it clears its mark and closes or quits again.

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{Emitter, Manager};

/// The event a window gets when closing it would lose its edits: ask before closing.
pub(crate) const CONFIRM_CLOSE_EVENT: &str = "window://confirm-close";

/// The event a window gets when quitting would lose its edits: ask before quitting.
pub(crate) const CONFIRM_QUIT_EVENT: &str = "window://confirm-quit";

/// The windows whose documents have unsaved edits, by label.
#[derive(Default)]
pub(crate) struct Unsaved(Mutex<BTreeSet<String>>);

impl Unsaved {
    /// Marks `window` as holding unsaved edits, or not.
    pub(crate) fn set(&self, window: &str, unsaved: bool) {
        let mut windows = self.lock();
        if unsaved {
            windows.insert(window.to_owned());
        } else {
            windows.remove(window);
        }
    }

    /// Whether `window` holds unsaved edits.
    pub(crate) fn holds(&self, window: &str) -> bool {
        self.lock().contains(window)
    }

    /// A window holding unsaved edits, when one does: the first by label.
    pub(crate) fn first(&self) -> Option<String> {
        self.lock().first().cloned()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether a close of `window` may go ahead; when it may not, asks the window first.
pub(crate) fn may_close(window: &tauri::Window) -> bool {
    if !window.state::<Unsaved>().holds(window.label()) {
        return true;
    }
    if let Err(error) = window.emit_to(window.label(), CONFIRM_CLOSE_EVENT, ()) {
        tracing::warn!("could not ask a window before closing it: {error}");
    }
    false
}

/// Whether the app may quit; when a window holds unsaved edits, brings it forward and asks it.
pub(crate) fn may_quit(app: &tauri::AppHandle) -> bool {
    let Some(label) = app.state::<Unsaved>().first() else {
        return true;
    };
    if let Some(window) = app.get_webview_window(&label) {
        // The question belongs to that window: show it in front.
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    if let Err(error) = app.emit_to(label.as_str(), CONFIRM_QUIT_EVENT, ()) {
        tracing::warn!("could not ask a window before quitting: {error}");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::Unsaved;

    #[test]
    fn a_window_marked_and_then_cleared_should_hold_nothing() {
        let unsaved = Unsaved::default();
        unsaved.set("main", true);
        unsaved.set("w2", true);
        unsaved.set("main", false);
        assert_eq!((unsaved.holds("main"), unsaved.first().as_deref()), (false, Some("w2")));
    }
}
