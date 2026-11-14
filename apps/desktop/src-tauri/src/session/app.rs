//! The session in the running app: loaded at launch ([`start`]), written every
//! [`SAVE_EVERY`](super::track::SAVE_EVERY) while it changed and when the app quits ([`flush`]),
//! deleted when the preferences stop keeping it, and the commands the windows call.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use laminapdf_desktop_protocol::DocId;
use tauri::Manager as _;
use tauri_plugin_window_state::{AppHandleExt as _, StateFlags};

use super::restore::{RestoredWindow, last_number, plan};
use super::track::{ReportedTab, SAVE_EVERY, Session, snapshot};
use super::{Bookmarks, Resolved, delete, load, resolve, save, should_persist, to_hex};
use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines};
use crate::prefs::{Preferences, Prefs};

/// What `tauri-plugin-window-state` keeps of each window: its size, position and maximised state.
pub(crate) const WINDOW_STATE: StateFlags = StateFlags::SIZE
    .union(StateFlags::POSITION)
    .union(StateFlags::MAXIMIZED);

/// The OS's bookmarks: macOS's, none elsewhere.
struct OsBookmarks;

impl Bookmarks for OsBookmarks {
    fn resolve(&self, bookmark: &[u8]) -> Option<PathBuf> {
        laminapdf_desktop_os::resolve_bookmark(bookmark)
    }
}

/// Each restored window's tabs, until its webview takes them.
#[derive(Default)]
pub(crate) struct Pending(Mutex<BTreeMap<String, RestoredWindow>>);

const fn keeps(prefs: Preferences) -> bool {
    should_persist(prefs.remember_recent, prefs.reopen_last)
}

/// Loads the session saved in `dir` when the preferences keep it (else deletes it), starts the
/// saver, and returns the windows to make again: none when there is no session.
pub(crate) fn start(app: &tauri::App, dir: PathBuf) -> Vec<RestoredWindow> {
    let session = Session::new(dir);
    let restored = if keeps(app.state::<Prefs>().get()) {
        restorable(&session)
    } else {
        if let Err(error) = delete(session.dir()) {
            tracing::warn!("could not delete the session file: {error}");
        }
        Vec::new()
    };
    app.state::<crate::windows::Windows>()
        .count_from(last_number(&restored));
    let pending = restored
        .iter()
        .map(|window| (window.label.clone(), window.clone()))
        .collect();
    app.manage(Pending(Mutex::new(pending)));
    app.manage(Arc::clone(&session));
    start_saver(app.handle().clone(), session);
    restored
}

/// The saved session's windows, its tabs kept in `session`'s tracker; none without a usable file.
fn restorable(session: &Session) -> Vec<RestoredWindow> {
    match load(session.dir()) {
        Ok(Some(file)) => plan(file, &OsBookmarks, &mut session.tracker()),
        Ok(None) => Vec::new(),
        Err(error) => {
            tracing::warn!("starting without the last session: {error}");
            Vec::new()
        }
    }
}

/// Writes the session every [`SAVE_EVERY`] while it changed.
fn start_saver(handle: tauri::AppHandle, session: Arc<Session>) {
    let spawned = std::thread::Builder::new().name("session-saver".into()).spawn(move || {
        loop {
            std::thread::sleep(SAVE_EVERY);
            if session.tracker().take_dirty() {
                flush(&handle);
            }
        }
    });
    if let Err(error) = spawned {
        tracing::warn!("the session will be saved only on quit: {error}");
    }
}

/// Writes the windows' sizes and places, and the session when the preferences keep it. The
/// window-state plugin saves on its own only when the app quits cleanly; saving with the session
/// keeps them after a crash or a forced quit too.
pub(crate) fn flush(app: &tauri::AppHandle) {
    if let Err(error) = app.save_window_state(WINDOW_STATE) {
        tracing::warn!("could not save the windows' sizes: {error}");
    }
    if !keeps(app.state::<Prefs>().get()) {
        return;
    }
    let session = app.state::<Arc<Session>>();
    let engines = Arc::clone(&app.state::<Arc<Engines>>());
    // The reports are copied under the lock; the file system is read outside it.
    let reports = session.tracker().reports();
    let mut bookmarks = session.bookmarks();
    let file = snapshot(
        &reports,
        crate::windows::window_order,
        |doc| engines.path_of(doc),
        |path| {
            bookmarks
                .entry(path.to_owned())
                .or_insert_with(|| laminapdf_desktop_os::file_bookmark(path).map(|bytes| to_hex(&bytes)))
                .clone()
        },
    );
    drop(bookmarks);
    if let Err(error) = save(session.dir(), &file) {
        tracing::warn!("could not save the session: {error}");
    }
}

/// The app is quitting: windows that close now stay in the session, which is written at once.
pub(crate) fn quitting(app: &tauri::AppHandle) {
    app.state::<Arc<Session>>().tracker().quitting();
    flush(app);
}

/// Window `label` closed. Closing the last window quits the app outside macOS, so it stays then.
pub(crate) fn window_closed(app: &tauri::AppHandle, label: &str) {
    app.state::<Arc<Session>>()
        .tracker()
        .closed(label, !cfg!(target_os = "macos"));
}

/// What a change of the preferences does to the session file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OnChange {
    /// They no longer keep it: it goes.
    Delete,
    /// They keep it again: the next save writes it.
    Resume,
    Nothing,
}

const fn on_change(was: Preferences, now: Preferences) -> OnChange {
    if !keeps(now) {
        OnChange::Delete
    } else if !keeps(was) {
        OnChange::Resume
    } else {
        OnChange::Nothing
    }
}

/// The preferences went from `was` to `now`: a session they no longer keep is deleted; one they
/// keep again is written at the next save.
pub(crate) fn preferences_changed(app: &tauri::AppHandle, was: Preferences, now: Preferences) {
    let session = app.state::<Arc<Session>>();
    match on_change(was, now) {
        OnChange::Delete => {
            if let Err(error) = delete(session.dir()) {
                tracing::warn!("could not delete the session file: {error}");
            }
        }
        OnChange::Resume => session.tracker().report_changed(),
        OnChange::Nothing => {}
    }
}

/// The most of a tab's view a report keeps: the viewer's is far smaller; more is dropped.
const MAX_VIEW: usize = 4096;

/// This window's tabs now, and the shown one's index: what a relaunch brings back.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and arguments by value"
)]
pub(crate) fn session_tabs(window: tauri::WebviewWindow, mut tabs: Vec<ReportedTab>, active: Option<usize>) {
    let label = window.label();
    let engines = window.state::<Arc<Engines>>();
    // A window reports only its own documents, and a view of a sensible size.
    let mut paths = Vec::new();
    for tab in &mut tabs {
        if let Some(doc) = tab.doc.map(DocId) {
            match engines.path_of(doc).filter(|_| engines.owns(label, doc)) {
                Some(path) => paths.push((doc, path)),
                None => tab.doc = None,
            }
        }
        if serde_json::to_string(&tab.view).map_or(true, |json| json.len() > MAX_VIEW) {
            tab.view = serde_json::Value::Null;
        }
    }
    let session = window.state::<Arc<Session>>();
    let mut tracker = session.tracker();
    for (doc, path) in paths {
        tracker.remember(doc, path);
    }
    tracker.report(label, tabs, active);
}

/// The tabs this window had in the last session, once; `None` for a window that had none.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn take_session_tabs(window: tauri::WebviewWindow) -> Option<RestoredWindow> {
    window
        .state::<Pending>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(window.label())
}

/// Opens restored tab `token`'s file in this window: where its bookmark finds it, else at its
/// path. A file neither finds fails as missing (`io`).
///
/// # Errors
/// [`DesktopError::Io`] for a missing file or an unknown token, and whatever opening it reports.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and arguments by value"
)]
pub(crate) fn session_open(window: tauri::WebviewWindow, token: String) -> Result<DocumentMeta, DesktopError> {
    let session = window.state::<Arc<Session>>();
    let tab = session
        .tracker()
        .restored_tab(&token, window.label())
        .cloned()
        .ok_or_else(missing)?;
    let path = match resolve(&tab, &OsBookmarks) {
        Resolved::Found(path) | Resolved::Moved(path) => path,
        Resolved::Missing => return Err(missing()),
    };
    let opened = crate::opened::open_path(window.app_handle(), window.label(), &path)?;
    let mut tracker = session.tracker();
    tracker.remember(DocId(opened.doc), path);
    tracker.opened(&token);
    Ok(opened)
}

fn missing() -> DesktopError {
    DesktopError::Io(std::io::Error::from(std::io::ErrorKind::NotFound))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefs(remember_recent: bool, reopen_last: bool) -> Preferences {
        Preferences {
            remember_recent,
            reopen_last,
            ..Preferences::default()
        }
    }

    #[test]
    fn turning_either_switch_off_should_delete_the_session() {
        let on = prefs(true, true);
        assert_eq!(
            [on_change(on, prefs(false, true)), on_change(on, prefs(true, false))],
            [OnChange::Delete, OnChange::Delete]
        );
    }

    #[test]
    fn turning_the_last_switch_back_on_should_resume_saving() {
        assert_eq!(
            [
                on_change(prefs(false, true), prefs(true, true)),
                on_change(prefs(true, true), prefs(true, true))
            ],
            [OnChange::Resume, OnChange::Nothing]
        );
    }
}
