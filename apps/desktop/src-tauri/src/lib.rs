//! Lamina PDF's desktop reader (ADR 0008): the Tauri 2 shell around `@laminapdf/ui`.
//!
//! The main process owns windows, dialogs and (in later todos) files; untrusted PDF bytes are
//! parsed only in the sandboxed engine process (D2b). The webview reaches Rust only through the
//! app commands in `commands`, each allowed by name in `capabilities/main.json`; the dialog, log and
//! opener plugins are for Rust's own use and grant the webview nothing. Desktop only: there is no
//! mobile entry point.

mod commands;
mod error;

pub use commands::AppInfo;
pub use error::DesktopError;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// The main window's label; later windows are `doc-*` (D4a).
const MAIN_WINDOW: &str = "main";

/// Builds and runs the app until its last window closes.
///
/// # Errors
/// [`DesktopError::Tauri`] when the app or its main window can't be created.
#[cfg_attr(
    not(target_os = "linux"),
    expect(
        clippy::disallowed_types,
        reason = "tauri::generate_context! builds a HashMap (not on Linux)"
    )
)]
pub fn run() -> Result<(), DesktopError> {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                // Stdout and the OS log folder. Document paths are logged at `debug`, never `info`.
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .setup(|app| {
            // Windows are made here, not in tauri.conf.json, so their labels stay ours (D4a).
            WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::default())
                .title("Lamina PDF")
                .inner_size(1200.0, 800.0)
                .min_inner_size(640.0, 480.0)
                .build()?;
            tracing::info!(version = %app.package_info().version, "Lamina PDF started");
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}
