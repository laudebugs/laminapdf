//! The session in the running app: loaded at launch ([`start`]), written every
//! [`SAVE_EVERY`](super::track::SAVE_EVERY) while it changed and when the app quits ([`flush`]),
//! deleted when the preferences stop keeping it, and the commands the windows call.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use tauri::Manager as _;

use super::restore::{RestoredWindow, last_number, plan};
use super::track::{ReportedTab, SAVE_EVERY, Session};
use super::{Bookmarks, Resolved, delete, load, resolve, save, should_persist, to_hex};
use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines};
use crate::prefs::{Preferences, Prefs};

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

fn keeps(prefs: Preferences) -> bool {
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

/// Writes the session now, when the preferences keep it.
pub(crate) fn flush(app: &tauri::AppHandle) {
    if !keeps(app.state::<Prefs>().get()) {
        return;
    }
    let session = app.state::<Arc<Session>>();
    let engines = Arc::clone(&app.state::<Arc<Engines>>());
    let file = session.tracker().snapshot(
        crate::windows::window_order,
        |doc| engines.path_of(doc),
        |path| laminapdf_desktop_os::file_bookmark(path).map(|bytes| to_hex(&bytes)),
    );
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

/// The preferences went from `was` to `now`: a session they no longer keep is deleted; one they
/// keep again is written at the next save.
pub(crate) fn preferences_changed(app: &tauri::AppHandle, was: Preferences, now: Preferences) {
    let session = app.state::<Arc<Session>>();
    if !keeps(now) {
        if let Err(error) = delete(session.dir()) {
            tracing::warn!("could not delete the session file: {error}");
        }
    } else if !keeps(was) {
        session.tracker().report_changed();
    }
}

/// This window's tabs now, and the shown one's index: what a relaunch brings back.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and arguments by value"
)]
pub(crate) fn session_tabs(window: tauri::WebviewWindow, tabs: Vec<ReportedTab>, active: Option<usize>) {
    let session = window.state::<Arc<Session>>();
    session.tracker().report(window.label(), tabs, active);
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
    let tab = window
        .state::<Arc<Session>>()
        .tracker()
        .restored_tab(&token)
        .cloned()
        .ok_or_else(missing)?;
    let path = match resolve(&tab, &OsBookmarks) {
        Resolved::Found(path) | Resolved::Moved(path) => path,
        Resolved::Missing => return Err(missing()),
    };
    crate::opened::open_path(window.app_handle(), window.label(), &path)
}

fn missing() -> DesktopError {
    DesktopError::Io(std::io::Error::from(std::io::ErrorKind::NotFound))
}
