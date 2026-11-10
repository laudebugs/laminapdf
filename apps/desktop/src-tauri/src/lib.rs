//! Lamina PDF's desktop reader (ADR 0008): the Tauri 2 shell around `@laminapdf/ui`.
//!
//! The main process owns windows, dialogs and (in later todos) files; untrusted PDF bytes are
//! parsed only in the sandboxed engine process (ADR 0008 §2). The webview reaches Rust only through the
//! app commands in `commands`, each allowed by name in `capabilities/main.json`; the dialog, log and
//! opener plugins are for Rust's own use and grant the webview nothing. Desktop only: there is no
//! mobile entry point.

mod commands;
mod error;

pub use error::DesktopError;
use tauri::{WebviewUrl, WebviewWindowBuilder};

/// The main window's label; a document's own windows will be `doc-*` (`capabilities/main.json`).
const MAIN_WINDOW: &str = "main";

/// The first window's size in logical pixels: 1200 × 800, shrunk to fit a smaller screen (a
/// 1366 × 768 laptop would otherwise lose the status bar under its taskbar), never below the
/// window's minimum.
fn first_window_size(monitor: Option<tauri::Monitor>) -> (f64, f64) {
    let Some(monitor) = monitor else {
        return (1200.0, 800.0);
    };
    let screen = monitor.work_area().size.to_logical::<f64>(monitor.scale_factor());
    fit_window((screen.width, screen.height))
}

/// 1200 × 800, or 90 % of `screen` when that is smaller, but at least 640 × 480.
fn fit_window(screen: (f64, f64)) -> (f64, f64) {
    (
        (screen.0 * 0.9).clamp(640.0, 1200.0),
        (screen.1 * 0.9).clamp(480.0, 800.0),
    )
}

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
            // Windows are made here, not in tauri.conf.json, so the app chooses every label.
            let (width, height) = first_window_size(app.primary_monitor().ok().flatten());
            WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::default())
                .title("Lamina PDF")
                .inner_size(width, height)
                .min_inner_size(640.0, 480.0)
                .center()
                .build()?;
            tracing::info!(version = %app.package_info().version, "Lamina PDF started");
            Ok(())
        })
        .run(tauri::generate_context!())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::fit_window;

    #[test]
    fn fit_window_should_keep_the_default_on_a_large_screen() {
        assert_eq!(fit_window((2560.0, 1440.0)), (1200.0, 800.0));
    }

    #[test]
    fn fit_window_should_shrink_to_a_small_screen() {
        assert_eq!(fit_window((1366.0, 728.0)), (1200.0, 655.2));
    }

    #[test]
    fn fit_window_should_not_go_below_the_minimum() {
        assert_eq!(fit_window((600.0, 400.0)), (640.0, 480.0));
    }
}
