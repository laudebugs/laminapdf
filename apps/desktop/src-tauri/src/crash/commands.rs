//! The webview's side of crash reporting: the question and its answer (S08), the Settings ›
//! Privacy switch and sample (S07), and the webview's own uncaught errors. The webview sees a
//! report only as the JSON that would be sent; it never sends one itself.

use std::sync::Arc;

use tauri::Manager as _;

use super::{CrashReports, Question};
use crate::crash::Crash;

/// The longest stack the webview may pass; the report keeps less.
const MAX_STACK: usize = 16 * 1024;

/// The question to ask about the newest crash, while the app may still ask; `null` otherwise.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn crash_question(reports: tauri::State<'_, Arc<CrashReports>>) -> Option<Question> {
    reports.question()
}

/// The user's answer about report `id`: `send` it or not, and `always` send from now on.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments and state by value"
)]
pub(crate) fn crash_answer(reports: tauri::State<'_, Arc<CrashReports>>, id: String, send: bool, always: bool) {
    reports.answer(&id, send, always);
}

/// Settings › Privacy: whether crash reports are sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CrashSettings {
    pub enabled: bool,
}

#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn crash_settings(reports: tauri::State<'_, Arc<CrashReports>>) -> CrashSettings {
    CrashSettings {
        enabled: reports.enabled(),
    }
}

/// Turns sending crash reports on or off.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn crash_set_enabled(reports: tauri::State<'_, Arc<CrashReports>>, enabled: bool) -> CrashSettings {
    reports.set_enabled(enabled);
    CrashSettings { enabled }
}

/// A sample report, as this app would send one ("See a sample report").
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its state by value")]
pub(crate) fn crash_sample(reports: tauri::State<'_, Arc<CrashReports>>) -> String {
    reports.sample()
}

/// An uncaught error (`rejection`: an unhandled rejection) in this window's page: the error's type
/// (`name`) and its `Error.stack` value. Kept like any crash, as its type and its frames' bundle
/// files; never its message.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments and state by value"
)]
pub(crate) fn report_ui_error(
    window: tauri::WebviewWindow,
    reports: tauri::State<'_, Arc<CrashReports>>,
    name: String,
    stack: String,
    rejection: bool,
) {
    let stack = laminapdf_desktop_protocol::crash::truncate_on_char_boundary(stack, MAX_STACK);
    reports.record(
        window.app_handle(),
        Some(window.label()),
        Crash::webview(&name, &stack, rejection),
    );
}

/// A main-process panic on the main thread, which ends the app as a real one there would: the
/// next launch asks about it.
#[cfg(feature = "e2e")]
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its app handle by value"
)]
pub(crate) fn e2e_crash_main_panic(app: tauri::AppHandle) -> Result<(), crate::DesktopError> {
    app.run_on_main_thread(|| {
        #[expect(clippy::panic, reason = "the end-to-end tests' forced main-process panic")]
        {
            panic!("a main-process panic the end-to-end tests asked for");
        }
    })
    .map_err(|error| crate::DesktopError::Internal(error.to_string()))
}

/// Makes this window's engine panic with `message`, which the test fills with what a report must
/// not hold.
#[cfg(feature = "e2e")]
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window, state and arguments by value"
)]
pub(crate) fn e2e_crash_engine_panic(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<crate::engine::Engines>>,
    message: String,
) {
    engines.debug_panic(window.label(), message);
}

/// Makes this window's engine fault, a real native crash that leaves a minidump (D7d).
#[cfg(feature = "e2e")]
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window and state by value"
)]
pub(crate) fn e2e_crash_engine_segv(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<crate::engine::Engines>>,
) {
    engines.debug_segv(window.label());
}
