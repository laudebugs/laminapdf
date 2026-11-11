//! The app commands the webview may call, each allowed by name in `capabilities/main.json`.
//!
//! The webview names documents by id and never sends a path; the calling window comes from
//! Tauri, not from the webview's arguments, so a window reaches only its own documents.

use std::sync::Arc;

use laminapdf_desktop_protocol::DocId;
use serde::Serialize;
use tauri_plugin_dialog::DialogExt as _;
use zeroize::Zeroizing;

use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines, PageMeta};

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
    AppInfo::current(&app.package_info().version.to_string())
}

/// Lets the user pick a PDF, opens it in this window's engine and returns its pages; `None` when
/// the user cancels. The dialog runs from Rust, so the path never reaches the webview.
#[tauri::command]
pub(crate) async fn open_dialog(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
) -> Result<Option<DocumentMeta>, DesktopError> {
    let engines = Arc::clone(&engines);
    // The dialog and the engine's replies block: off the async runtime's threads.
    tauri::async_runtime::spawn_blocking(move || {
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
        engines.open(window.label(), &path).map(Some)
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
    engines.close(window.label(), DocId(doc))
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
