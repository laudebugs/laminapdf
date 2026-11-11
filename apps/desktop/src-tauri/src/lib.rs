//! Lamina PDF's desktop reader (ADR 0008): the Tauri 2 shell around `@laminapdf/ui`.
//!
//! The main process owns windows, dialogs and (in later todos) files; untrusted PDF bytes are
//! parsed only in the sandboxed engine process (ADR 0008 §2). The webview reaches Rust only through the
//! app commands in `commands`, each allowed by name in `capabilities/main.json`; the dialog, log and
//! opener plugins are for Rust's own use and grant the webview nothing. Desktop only: there is no
//! mobile entry point.

mod commands;
mod engine;
mod error;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

pub use error::DesktopError;
use laminapdf_desktop_protocol::supervisor::{Config, FontServer, Spawner};
use laminapdf_desktop_sandbox::{BackgroundFonts, SandboxConfig, SandboxSpawner};
use tauri::{Emitter as _, EventTarget, Manager as _, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::engine::{Engines, FAILED_EVENT, Failed, Launcher, Notify, RESTARTED_EVENT, Restarted};

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

/// The engine binary: a Tauri sidecar, which Tauri places next to the app's own executable (in
/// `tauri dev` too). Missing in a plain `cargo build`: documents then fail to open with
/// `engine-start` until `cargo xtask desktop-sidecar` has run.
fn engine_program() -> PathBuf {
    let name = format!("laminapdf-desktop-engine{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&name)))
        .unwrap_or_else(|| PathBuf::from(name))
}

/// The engine's arguments: the PDFium folder among the app's resources, when it is there.
fn engine_args(app: &tauri::AppHandle) -> Vec<OsString> {
    match app.path().resource_dir().map(|dir| dir.join("pdfium")) {
        Ok(dir) if dir.is_dir() => vec!["--pdfium".into(), dir.into_os_string()],
        _ => {
            tracing::warn!("PDFium is not among the app's resources; pages won't render");
            Vec::new()
        }
    }
}

/// Sends engine events to the one window they concern.
struct WindowEvents(tauri::AppHandle);

impl Notify for WindowEvents {
    fn restarted(&self, payload: Restarted) {
        let target = EventTarget::webview_window(payload.window.clone());
        if let Err(error) = self.0.emit_to(target, RESTARTED_EVENT, payload) {
            tracing::warn!("could not tell a window its engine restarted: {error}");
        }
    }

    fn failed(&self, payload: Failed) {
        let target = EventTarget::webview_window(payload.window.clone());
        if let Err(error) = self.0.emit_to(target, FAILED_EVENT, payload) {
            tracing::warn!("could not tell a window its engine failed: {error}");
        }
    }
}

/// Every window's engine, started on first use.
fn engines(app: &tauri::AppHandle) -> Arc<Engines> {
    let (program, args) = (engine_program(), engine_args(app));
    let launcher: Launcher = Arc::new(move || {
        // Inside its sandbox (ADR 0008 §5); the supervisor logs the layers each engine reports.
        let spawner = SandboxSpawner::new(program.clone(), args.clone(), SandboxConfig::default(), |line| {
            tracing::info!(target: "laminapdf_desktop::engine", "{line}");
        });
        Box::new(spawner) as Box<dyn Spawner>
    });
    // The sandboxed engine can't open font files: the main process lists the system's faces and
    // serves them (ADR 0008 §5, fonts). Windows engines keep PDFium's GDI font mapper.
    let fonts: Option<Arc<dyn FontServer>> = if cfg!(windows) {
        None
    } else {
        Some(Arc::new(BackgroundFonts::start()))
    };
    let config = Config {
        fonts,
        ..Config::default()
    };
    Engines::new(launcher, config, Arc::new(WindowEvents(app.clone())))
}

/// Builds and runs the app until its last window closes.
///
/// # Errors
/// [`DesktopError::Tauri`] when the app or its main window can't be created.
#[expect(
    clippy::disallowed_types,
    reason = "tauri::generate_context! builds a HashMap of the bundled assets"
)]
pub fn run() -> Result<(), DesktopError> {
    // `debug_kill_engine` exists only in e2e builds; generate_handler! takes no cfg per command.
    #[cfg(not(feature = "e2e"))]
    let builder = tauri::Builder::default().invoke_handler(tauri::generate_handler![
        commands::app_info,
        commands::open_dialog,
        commands::page_info,
        commands::close_document,
    ]);
    #[cfg(feature = "e2e")]
    let builder = tauri::Builder::default().invoke_handler(tauri::generate_handler![
        commands::app_info,
        commands::open_dialog,
        commands::page_info,
        commands::close_document,
        commands::debug_kill_engine,
    ]);
    let app = builder
        .plugin(
            tauri_plugin_log::Builder::new()
                // Stdout and the OS log folder. Document paths are logged at `debug`, never `info`.
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(engines(app.handle()));
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
        .on_window_event(|window, event| {
            // A closed window's documents close, their passwords are wiped, its engine stops.
            if matches!(event, WindowEvent::Destroyed) {
                window.state::<Arc<Engines>>().close_window(window.label());
            }
        })
        .build(tauri::generate_context!())?;
    app.run(|app, event| {
        if matches!(event, RunEvent::Exit) {
            app.state::<Arc<Engines>>().shutdown();
        }
    });
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
        let (width, height) = fit_window((1366.0, 728.0));
        assert!(
            (width - 1200.0).abs() < 1e-9 && (height - 655.2).abs() < 1e-9,
            "{width} × {height}"
        );
    }

    #[test]
    fn fit_window_should_not_go_below_the_minimum() {
        assert_eq!(fit_window((600.0, 400.0)), (640.0, 480.0));
    }
}
