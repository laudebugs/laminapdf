//! The app commands the webview may call, each allowed by name in `capabilities/main.json`.
//!
//! The webview names documents by id, and recent files by their entry's id, and never sends a
//! path; the calling window comes from Tauri, not from the webview's arguments, so a window reaches
//! only its own documents.

use std::sync::Arc;

use laminapdf_desktop_protocol::DocId;
use serde::Serialize;
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use zeroize::Zeroizing;

use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines, PageMeta};
use crate::opened::{self, open_path};
use crate::prefs::{Preferences, Prefs};
use crate::recent::{self, RecentFiles};

/// What `app_info` reports: enough for an About box and a bug report, nothing about documents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    /// The app's version (the workspace's lockstep version, ADR 0017).
    pub version: String,
    /// `macos`, `windows` or `linux`.
    pub os: &'static str,
    /// `aarch64`, `x86_64` …
    pub arch: &'static str,
    /// The system webview's version, when the platform reports it.
    pub webview_version: Option<String>,
}

impl AppInfo {
    /// This build on this machine.
    #[must_use]
    pub(crate) fn current(version: &str) -> Self {
        Self {
            version: version.to_owned(),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            webview_version: tauri::webview_version().ok(),
        }
    }
}

/// The app's version, OS, CPU and webview version: proves the IPC and permission plumbing.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its AppHandle by value"
)]
pub(crate) fn app_info(app: tauri::AppHandle) -> AppInfo {
    #[cfg(feature = "e2e")]
    crate::startup::app_info();
    AppInfo::current(&app.package_info().version.to_string())
}

/// Lets the user pick a PDF, opens it in this window's engine and returns its pages; `None` when
/// the user cancels. The dialog runs from Rust, so the path never reaches the webview.
#[tauri::command]
pub(crate) async fn open_dialog(window: tauri::WebviewWindow) -> Result<Option<DocumentMeta>, DesktopError> {
    // The dialog and the engine's replies block: off the async runtime's threads.
    tauri::async_runtime::spawn_blocking(move || {
        // End-to-end builds: a scenario's queued fixture stands in for the dialog.
        #[cfg(feature = "e2e")]
        if let Some(path) = tauri::Manager::state::<crate::e2e::QueuedPick>(&window).take() {
            return open_path(window.app_handle(), window.label(), &path).map(Some);
        }
        let picked = window
            .dialog()
            .file()
            .set_parent(&window)
            .add_filter("PDF", &["pdf"])
            .blocking_pick_file();
        let Some(picked) = picked else {
            return Ok(None);
        };
        let path = picked
            .into_path()
            .map_err(|error| DesktopError::Internal(error.to_string()))?;
        open_path(window.app_handle(), window.label(), &path).map(Some)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Every page's size of one of this window's documents.
#[tauri::command]
pub(crate) async fn page_info(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
) -> Result<Vec<PageMeta>, DesktopError> {
    let engines = Arc::clone(&engines);
    tauri::async_runtime::spawn_blocking(move || engines.page_info(window.label(), DocId(doc)))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Unlocks one of this window's documents with the password the user typed; the main process
/// keeps it while the document stays open, so a restarted engine never asks again. The command's
/// own copy is wiped when dropped and never logged; the IPC layer's buffers and serde's copies
/// along the way are not ours to wipe.
#[tauri::command]
pub(crate) async fn unlock_document(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    password: String,
) -> Result<DocumentMeta, DesktopError> {
    let password = Zeroizing::new(password);
    let engines = Arc::clone(&engines);
    tauri::async_runtime::spawn_blocking(move || engines.unlock(window.label(), DocId(doc), &password))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Drops tiles this window's viewer no longer wants (scrolled away, zoomed past), by the tickets
/// their `lamina://` addresses carried.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and state by value"
)]
pub(crate) fn cancel_tiles(window: tauri::WebviewWindow, engines: tauri::State<'_, Arc<Engines>>, tickets: Vec<u64>) {
    engines.cancel_tiles(window.label(), &tickets);
}

/// Closes one of this window's documents: the engine forgets it, its file closes and its kept
/// password is wiped.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and state by value"
)]
pub(crate) fn close_document(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
) -> Result<(), DesktopError> {
    engines.close(window.label(), DocId(doc))?;
    opened::stop_watching(window.app_handle(), window.label(), DocId(doc));
    Ok(())
}

/// The recent files, newest first: ids, names, sizes and dates, never a path.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn recent_list(recent: tauri::State<'_, RecentFiles>) -> Vec<recent::Listed> {
    recent.list()
}

/// Opens the recent file `id` in this window, as the open dialog would; the webview names the
/// entry, never its path.
#[tauri::command]
pub(crate) async fn recent_open(window: tauri::WebviewWindow, id: String) -> Result<DocumentMeta, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = window
            .state::<RecentFiles>()
            .path_of(&id)
            .ok_or_else(|| DesktopError::Io(std::io::ErrorKind::NotFound.into()))?;
        open_path(window.app_handle(), window.label(), &path)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Forgets the recent file `id`, in the app's list and, where the OS can, in its own.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its handle and state by value"
)]
pub(crate) fn recent_remove(app: tauri::AppHandle, recent: tauri::State<'_, RecentFiles>, id: String) {
    if let Some(path) = recent.remove(&id) {
        opened::forget(&app, vec![path], false);
    }
}

/// Forgets every recent file: the app's list and the OS's list of this app's documents.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its handle and state by value"
)]
pub(crate) fn recent_clear(app: tauri::AppHandle, recent: tauri::State<'_, RecentFiles>) {
    opened::forget(&app, recent.clear(), true);
}

/// The app's preferences.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn preferences(prefs: tauri::State<'_, Prefs>) -> Preferences {
    prefs.get()
}

/// Sets the app's preferences. Turning "Remember recent files" off clears both recent lists.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its handle and state by value"
)]
pub(crate) fn set_preferences(
    app: tauri::AppHandle,
    prefs: tauri::State<'_, Prefs>,
    recent: tauri::State<'_, RecentFiles>,
    value: Preferences,
) {
    let was = prefs.get();
    prefs.set(value);
    if was.stops_remembering(value) {
        opened::forget(&app, recent.clear(), true);
    }
}

/// Kills this window's engine as a crash would (end-to-end tests only, feature `e2e`).
#[cfg(feature = "e2e")]
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and state by value"
)]
pub(crate) fn debug_kill_engine(window: tauri::WebviewWindow, engines: tauri::State<'_, Arc<Engines>>) {
    engines.kill(window.label());
}

#[cfg(test)]
mod tests {
    use super::AppInfo;
    use crate::DesktopError;

    /// The sample the frontend's test answers `app_info` with (apps/desktop/test/app-info.json), so
    /// the Rust struct and the TypeScript `AppInfo` agree on the JSON.
    const SAMPLE: &str = include_str!("../../test/app-info.json");

    #[test]
    fn app_info_should_serialise_as_the_frontend_expects() {
        let info = AppInfo {
            version: "1.2.3".into(),
            os: "macos",
            arch: "aarch64",
            webview_version: Some("1".into()),
        };
        let expected: serde_json::Value = serde_json::from_str(SAMPLE).unwrap_or_default();
        assert_eq!(serde_json::to_value(&info).unwrap_or_default(), expected);
    }

    #[test]
    fn desktop_error_should_serialise_as_kind_and_message() {
        let error = DesktopError::Tauri(tauri::Error::WebviewNotFound);
        let json = serde_json::to_value(&error).unwrap_or_default();
        assert_eq!(json["kind"], "tauri");
        assert!(
            json["message"]
                .as_str()
                .is_some_and(|m| m.starts_with("the desktop shell failed"))
        );
    }
}

/// Opens a new, empty window (File ▸ New Window) and returns its label.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its app by value")]
pub(crate) fn new_window(app: tauri::AppHandle) -> Result<String, DesktopError> {
    crate::windows::new_window(&app)
}

/// Moves one of this window's tabs (its document `doc`, with `view`, the page, offset and zoom it
/// showed) to a new window or another one, and returns the label of the window it went to. The
/// document keeps its kept password, so a moved tab never asks for it again; on failure the tab
/// stays here.
#[tauri::command]
pub(crate) async fn move_tab(
    window: tauri::WebviewWindow,
    doc: u32,
    to: crate::windows::MoveTo,
    view: crate::windows::View,
) -> Result<String, DesktopError> {
    let app = window.app_handle().clone();
    let from = window.label().to_owned();
    // The engines' answers come from other processes: never on the main thread.
    tauri::async_runtime::spawn_blocking(move || crate::windows::move_tab(&app, &from, DocId(doc), &to, view))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// The other windows and their titles, for "Move tab to window ▸".
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn window_list(window: tauri::WebviewWindow) -> Vec<crate::windows::WindowEntry> {
    crate::windows::others(window.app_handle(), window.label())
}

/// Sets this window's title from the shown document's name (`None`: the start screen).
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn set_window_title(window: tauri::WebviewWindow, name: Option<String>) -> Result<(), DesktopError> {
    window.set_title(&crate::windows::title(name.as_deref()))?;
    Ok(())
}

/// Hands this window's webview the tabs moved to it before it was ready, through
/// `tab://arrived`, and lets later ones through at once. The webview calls it once, after it
/// listens for the event.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn take_pending_tabs(window: tauri::WebviewWindow) {
    crate::windows::take_arrivals(window.app_handle(), window.label());
}
