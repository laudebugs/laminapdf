//! The app commands the webview may call, each allowed by name in `capabilities/main.json`.
//!
//! The webview names documents by id, and recent files by their entry's id, and never sends a
//! path; the calling window comes from Tauri, not from the webview's arguments, so a window reaches
//! only its own documents.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use laminapdf_desktop_protocol::{DocId, Query};
use serde::Serialize;
use tauri::{Emitter as _, Manager as _};
use tauri_plugin_dialog::DialogExt as _;
use zeroize::Zeroizing;

use crate::DesktopError;
use crate::engine::{DocumentMeta, EditOutcome, EditRequest, Engines, PageMeta};
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
    tauri::async_runtime::spawn_blocking(move || match pick_pdf(&window)? {
        Some(path) => open_path(window.app_handle(), window.label(), &path).map(Some),
        None => Ok(None),
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// The PDF the user picks in the open dialog over `window`, or `None` when they cancel. It
/// blocks until they answer: never on the main thread.
pub(crate) fn pick_pdf(window: &tauri::WebviewWindow) -> Result<Option<PathBuf>, DesktopError> {
    pick_pdf_in(window, None)
}

/// [`pick_pdf`], starting in `folder` when there is one.
fn pick_pdf_in(window: &tauri::WebviewWindow, folder: Option<&Path>) -> Result<Option<PathBuf>, DesktopError> {
    // End-to-end builds: a scenario's queued fixture stands in for the dialog.
    #[cfg(feature = "e2e")]
    if let Some(path) = tauri::Manager::state::<crate::e2e::QueuedPick>(window).take() {
        return Ok(Some(path));
    }
    let mut dialog = window.dialog().file().set_parent(window).add_filter("PDF", &["pdf"]);
    if let Some(folder) = folder {
        dialog = dialog.set_directory(folder);
    }
    let picked = dialog.blocking_pick_file();
    picked
        .map(tauri_plugin_dialog::FilePath::into_path)
        .transpose()
        .map_err(|error| DesktopError::Internal(error.to_string()))
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

/// What the Properties dialog shows of one of this window's documents, with its edits so far: the
/// engine's JSON (`FromEngine::Properties`), which the webview reads as it is.
#[tauri::command]
pub(crate) async fn document_properties(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
) -> Result<String, DesktopError> {
    let engines = Arc::clone(&engines);
    tauri::async_runtime::spawn_blocking(move || engines.properties(window.label(), DocId(doc)))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// What saving one of this window's documents would break if the op log `log` were applied
/// first: the engine's `Violation[]` JSON, which the Organise mode asks before each edit.
#[tauri::command]
pub(crate) async fn would_violate(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    log: String,
) -> Result<String, DesktopError> {
    query(window, &engines, doc, Query::WouldViolate { log }).await
}

/// Page `index`'s boxes in user space, as the document's edits leave them, for Page boxes…: the
/// engine's `PageBoxesRead` JSON.
#[tauri::command]
pub(crate) async fn page_boxes(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    index: u32,
) -> Result<String, DesktopError> {
    query(window, &engines, doc, Query::PageBoxes { index }).await
}

/// The document's page label ranges as its edits leave them, for Page labels…: the engine's
/// `PageLabelRange[]` JSON.
#[tauri::command]
pub(crate) async fn label_ranges(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
) -> Result<String, DesktopError> {
    query(window, &engines, doc, Query::LabelRanges).await
}

/// Asks the window's engine `query` about `doc` off the async runtime's threads.
async fn query(
    window: tauri::WebviewWindow,
    engines: &Arc<Engines>,
    doc: u32,
    query: Query,
) -> Result<String, DesktopError> {
    let engines = Arc::clone(engines);
    tauri::async_runtime::spawn_blocking(move || engines.query(window.label(), DocId(doc), query))
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
/// entry, never its path. An entry the sandbox can't reach again asks for the file in the picker,
/// starting in its folder; `None` when the user cancels.
#[tauri::command]
pub(crate) async fn recent_open(
    window: tauri::WebviewWindow,
    id: String,
) -> Result<Option<DocumentMeta>, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || {
        let way = window
            .state::<RecentFiles>()
            .way_to_open(&id)
            .ok_or_else(|| DesktopError::Io(std::io::ErrorKind::NotFound.into()))?;
        let path = match way {
            recent::Way::Open(path) => path,
            recent::Way::Choose(folder) => {
                // A second press while the picker is open opens no second picker.
                let Some(_picking) = recent::Picking::begin(window.label()) else {
                    return Ok(None);
                };
                match pick_pdf_in(&window, folder.as_deref())? {
                    Some(path) => path,
                    None => return Ok(None),
                }
            }
        };
        open_path(window.app_handle(), window.label(), &path).map(Some)
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

/// The preferences changed, with their new value: every window reads it. The event's name is
/// repeated in `src/settings.tsx`.
pub(crate) const PREFERENCES_EVENT: &str = "preferences://changed";

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
    let was = prefs.replace(value);
    if was.theme != value.theme {
        crate::windows::apply_theme_to_all(&app, value.theme);
    }
    // Every window shows them (Settings, Theme, Motion), not only the one that changed them.
    if let Err(error) = app.emit(PREFERENCES_EVENT, value) {
        tracing::warn!("could not tell the windows the preferences changed: {error}");
    }
    if was.stops_remembering(value) {
        opened::forget(&app, recent.clear(), true);
    }
    crate::session::app::preferences_changed(&app, was, value);
}

/// Settings › Graphics: what this start runs with, and whether the crash-loop guard switched it.
#[tauri::command]
pub(crate) fn graphics_status() -> crate::graphics::Status {
    crate::graphics::status()
}

/// Whether to tell the user the crash-loop guard switched to Compatibility: `true` once per run,
/// to the first window that asks, which then says so at start.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its handle by value")]
pub(crate) fn graphics_take_notice(app: tauri::AppHandle) -> Result<bool, DesktopError> {
    Ok(crate::graphics::take_notice(&app.path().app_config_dir()?))
}

/// Settings › About's View licences: `false` when this build bundles none.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its handle by value")]
pub(crate) fn open_licences(app: tauri::AppHandle) -> bool {
    crate::menu::open_licences(&app)
}

/// Restart now, after a change that takes effect at the next start (Settings › Graphics):
/// `false`, and no restart, while a window holds unsaved edits.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its handle by value")]
pub(crate) fn restart_app(app: tauri::AppHandle) -> bool {
    if app.state::<crate::unsaved::Unsaved>().to_ask().is_some() {
        return false;
    }
    crate::restart(&app)
}

/// Whether the system asks for reduced motion where the webview can't tell (`motion.rs`): `None`
/// where the webview reads it itself, or the setting can't be read.
#[tauri::command]
pub(crate) async fn system_motion() -> Option<bool> {
    tauri::async_runtime::spawn_blocking(crate::motion::system_reduces_motion)
        .await
        .ok()
        .flatten()
}

/// This window's commands' state for the native menus: which are enabled, and which toggles and
/// choices are on. The menus show the focused window's (src/menus/use-native-menus.tsx sends it).
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn menu_state(window: tauri::WebviewWindow, state: Vec<crate::menu::CommandState>) {
    crate::menu::set_state(window.app_handle(), window.label(), state);
}

/// Closes this window (File ▸ Close window): as its close button does, its documents with it.
/// Async, as every command that makes or closes a window: on Windows a synchronous command runs
/// on the main thread, which the window's teardown waits for.
#[tauri::command]
pub(crate) async fn close_window(window: tauri::WebviewWindow) -> Result<(), DesktopError> {
    window.close()?;
    Ok(())
}

/// Quits the app (File ▸ Quit, or the app menu's on macOS), unless a window holds unsaved edits:
/// that window is asked first. It quits again with `discard` once the user chose to lose them,
/// and the next window with edits is asked, until none is left.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn quit(window: tauri::WebviewWindow, discard: Option<bool>) {
    let app = window.app_handle();
    if discard == Some(true) {
        window.state::<crate::unsaved::Unsaved>().agree(window.label());
    }
    if crate::unsaved::may_quit(app) {
        app.exit(0);
    }
}

/// The user cancelled a quit in this window: every window with unsaved edits is asked again next
/// time.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn cancel_quit(window: tauri::WebviewWindow) {
    window.state::<crate::unsaved::Unsaved>().cancel_quit();
}

/// Says whether this window's documents hold edits not yet saved, so closing it or quitting asks
/// first, and which ones (`docs`), so a change another app makes to one of their files asks before
/// it reloads.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn set_unsaved(window: tauri::WebviewWindow, unsaved: bool, docs: Option<Vec<u32>>) {
    let marks = window.state::<crate::unsaved::Unsaved>();
    marks.set(window.label(), unsaved);
    marks.set_docs(window.label(), docs.as_deref().unwrap_or_default());
}

/// Applies, undoes or redoes an edit of one of this window's documents: the engine draws its
/// pages from the edited file from then on.
#[tauri::command]
pub(crate) async fn edit_document(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    request: EditRequest,
) -> Result<EditOutcome, DesktopError> {
    let engines = Arc::clone(&engines);
    tauri::async_runtime::spawn_blocking(move || engines.edit(window.label(), DocId(doc), request))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Saves a copy of this window's document `doc` where the user chooses (File ▸ Save a copy…):
/// the file's bytes as they are on disk, copied by the main process, never through the webview.
/// Returns the copy's file name, or `None` when the dialog was cancelled.
#[tauri::command]
pub(crate) async fn save_copy(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
) -> Result<Option<String>, DesktopError> {
    let from = engines
        .path_in(window.label(), DocId(doc))
        .ok_or_else(|| DesktopError::Internal("save_copy: not one of this window's documents".to_owned()))?;
    // The dialog blocks: off the async runtime's threads.
    tauri::async_runtime::spawn_blocking(move || {
        let name = from.file_name().map(|name| name.to_string_lossy().into_owned());
        let mut dialog = window.dialog().file().set_parent(&window).add_filter("PDF", &["pdf"]);
        if let Some(name) = &name {
            dialog = dialog.set_file_name(name);
        }
        let Some(to) = dialog.blocking_save_file() else {
            return Ok(None);
        };
        let to = to
            .into_path()
            .map_err(|error| DesktopError::Internal(error.to_string()))?;
        if to != from {
            std::fs::copy(&from, &to)?;
        }
        Ok(to.file_name().map(|name| name.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
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

/// Opens a new, empty window (File ▸ New Window) and returns its label. Async: on Windows a
/// synchronous command runs on the main thread, where making a window deadlocks.
#[tauri::command]
pub(crate) async fn new_window(app: tauri::AppHandle) -> Result<String, DesktopError> {
    crate::windows::new_window(&app, None)
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
    tauri::async_runtime::spawn_blocking(move || crate::windows::move_tab(&app, &from, DocId(doc), &to, view, None))
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// One of this window's tabs, its document `doc`, began to be dragged: until the drag ends, the
/// window under the cursor hears that a tab may drop on it (`tab://drag-over`).
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn tab_drag_start(window: tauri::WebviewWindow, doc: u32) {
    crate::tab_drag::start(window.app_handle(), window.label(), DocId(doc));
}

/// The drag of this window's tab of `doc` was cancelled: the tab stays, and Rust stops following
/// the cursor.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its window by value")]
pub(crate) fn tab_drag_cancel(window: tauri::WebviewWindow, doc: u32) {
    crate::tab_drag::cancel(window.app_handle(), window.label(), DocId(doc));
}

/// The drag of this window's tab of `doc` ended without a drop on this window's strip (`outside`:
/// the pointer had left the window, by the webview's account). Where the cursor is decides: over
/// another window the tab moves there, outside every window it opens in a new one under the
/// cursor, both with `view`. Returns the label of the window the tab went to, or `null` when it
/// stays; on failure it stays too.
#[tauri::command]
pub(crate) async fn tab_drag_end(
    window: tauri::WebviewWindow,
    doc: u32,
    view: crate::windows::View,
    outside: bool,
) -> Result<Option<String>, DesktopError> {
    let app = window.app_handle().clone();
    let from = window.label().to_owned();
    // A move waits on the engines' answers: never on the main thread.
    tauri::async_runtime::spawn_blocking(move || crate::tab_drag::end(&app, &from, DocId(doc), view, outside))
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
    crate::windows::changed(window.app_handle());
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
