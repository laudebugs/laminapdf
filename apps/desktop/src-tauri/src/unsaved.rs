//! The close and quit guard: a window whose document has edits not yet saved says so, and closing
//! it, or quitting the app, asks it first. The webview shows the question. To close, a window that
//! may lose its edits clears its mark and closes again. To quit, it quits again saying so, and the
//! next window with edits is asked; a Cancel in any of them forgets those answers, so each window
//! keeps its mark until the app really quits.

use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{Emitter, Manager};

/// The event a window gets when closing it would lose its edits: ask before closing.
pub(crate) const CONFIRM_CLOSE_EVENT: &str = "window://confirm-close";

/// The event a window gets when quitting would lose its edits: ask before quitting.
pub(crate) const CONFIRM_QUIT_EVENT: &str = "window://confirm-quit";

/// The windows whose documents have unsaved edits, by label, and those whose edits the quit
/// asking now may lose.
#[derive(Default)]
pub(crate) struct Unsaved(Mutex<Marks>);

#[derive(Default)]
struct Marks {
    unsaved: BTreeSet<String>,
    agreed: BTreeSet<String>,
}

impl Unsaved {
    /// Marks `window` as holding unsaved edits, or not.
    pub(crate) fn set(&self, window: &str, unsaved: bool) {
        let mut marks = self.lock();
        if unsaved {
            marks.unsaved.insert(window.to_owned());
        } else {
            marks.unsaved.remove(window);
            marks.agreed.remove(window);
        }
    }

    /// Whether `window` holds unsaved edits.
    pub(crate) fn holds(&self, window: &str) -> bool {
        self.lock().unsaved.contains(window)
    }

    /// `window`'s user chose to lose its edits to the quit asking now.
    pub(crate) fn agree(&self, window: &str) {
        self.lock().agreed.insert(window.to_owned());
    }

    /// The quit asking now was cancelled: the next one asks every window again.
    pub(crate) fn cancel_quit(&self) {
        self.lock().agreed.clear();
    }

    /// The first window, by label, holding unsaved edits that the quit hasn't been told it may
    /// lose; with none, the quit may go ahead, and its answers are forgotten.
    pub(crate) fn to_ask(&self) -> Option<String> {
        let mut marks = self.lock();
        let next = marks.unsaved.difference(&marks.agreed).next().cloned();
        if next.is_none() {
            marks.agreed.clear();
        }
        next
    }

    fn lock(&self) -> MutexGuard<'_, Marks> {
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
    let Some(label) = app.state::<Unsaved>().to_ask() else {
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
        assert_eq!(
            (unsaved.holds("main"), unsaved.to_ask().as_deref()),
            (false, Some("w2"))
        );
    }

    #[test]
    fn a_quit_should_ask_each_window_once_and_go_ahead_after_the_last() {
        let unsaved = Unsaved::default();
        unsaved.set("main", true);
        unsaved.set("w2", true);
        let first = unsaved.to_ask();
        unsaved.agree("main");
        let second = unsaved.to_ask();
        unsaved.agree("w2");
        assert_eq!(
            (first.as_deref(), second.as_deref(), unsaved.to_ask()),
            (Some("main"), Some("w2"), None)
        );
    }

    #[test]
    fn a_cancelled_quit_should_leave_every_window_marked_and_asked_again() {
        let unsaved = Unsaved::default();
        unsaved.set("main", true);
        unsaved.set("w2", true);
        unsaved.agree("main");
        unsaved.cancel_quit();
        assert_eq!(
            (unsaved.holds("main"), unsaved.to_ask().as_deref()),
            (true, Some("main"))
        );
    }
}
