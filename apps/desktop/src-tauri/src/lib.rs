//! Lamina PDF's desktop reader (ADR 0008): the Tauri 2 shell around `@laminapdf/ui`.
//!
//! The main process owns windows, dialogs and files (opening, the recent lists, watching for
//! changes); untrusted PDF bytes are parsed only in the sandboxed engine process (ADR 0008 §2).
//! The webview reaches Rust only through the app commands in `commands`, each allowed by name in
//! `capabilities/main.json`; the dialog, log and opener plugins are for Rust's own use and grant
//! the webview nothing. Desktop only: there is no
//! mobile entry point.

mod commands;
#[cfg(feature = "e2e")]
mod e2e;
mod engine;
mod error;
mod menu;
mod open;
mod opened;
mod prefs;
mod recent;
mod session;
#[cfg(feature = "e2e")]
mod startup;
mod store;
mod watch;
mod windows;

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

pub use error::DesktopError;
use laminapdf_desktop_protocol::supervisor::{Config, FontServer, Spawner};
use laminapdf_desktop_sandbox::{BackgroundFonts, SandboxConfig, SandboxSpawner};
use tauri::{Emitter as _, EventTarget, Manager as _, RunEvent, WindowEvent};

use crate::engine::{Engines, FAILED_EVENT, Failed, Launcher, Notify, RESTARTED_EVENT, Restarted};

/// The scheme the viewer fetches tiles from: `lamina://localhost/tile/…`, and
/// `http://lamina.localhost/tile/…` on Windows (the CSP allows both).
const TILE_SCHEME: &str = "lamina";

/// The main window's label; a document's own windows will be `doc-*` (`capabilities/main.json`).
pub(crate) const MAIN_WINDOW: &str = "main";

/// The first window's size in logical pixels: 1200 × 800, shrunk to fit a smaller screen (a
/// 1366 × 768 laptop would otherwise lose the status bar under its taskbar), never below the
/// window's minimum.
pub(crate) fn first_window_size(monitor: Option<tauri::Monitor>) -> (f64, f64) {
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

/// The app's state and its first window, before the event loop runs: the preferences and recent
/// list from the config folder, the engines, the file watcher, the OS's open requests.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "e2e")]
    startup::setup();
    // Windows: the taskbar's Jump List is this app ID's (the installers' shortcuts carry it).
    if let Err(error) = laminapdf_desktop_os::set_app_id(&app.config().identifier) {
        tracing::warn!("could not set the app's ID: {error}");
    }
    let config = app.path().app_config_dir()?;
    app.manage(prefs::Prefs::load(config.join("preferences.json")));
    app.manage(recent::RecentFiles::load(config.join("recent.json")));
    app.manage(engines(app.handle()));
    app.manage(watch::Watcher::start(Arc::new(watch::app::WindowReload(
        app.handle().clone(),
    )))?);
    app.manage(open::OpenWorker::start(app.handle().clone())?);
    app.manage(windows::Windows::default());
    app.manage(menu::MenusJson::embedded());
    app.manage(menu::NativeMenus::default());
    // The last session's windows (D4d), else one window. Windows are made here, not in
    // tauri.conf.json, so the app chooses every label.
    let restored = session::app::start(app, config);
    let labels: Vec<String> = if restored.is_empty() {
        vec![MAIN_WINDOW.to_owned()]
    } else {
        restored.into_iter().map(|window| window.label).collect()
    };
    for label in &labels {
        windows::build_window(app.handle(), label)?;
    }
    // The menu bar (macOS) or each window's menus (Windows, Linux), from the shared shortcut map.
    menu::rebuild(app.handle());
    #[cfg(feature = "e2e")]
    startup::window();
    tracing::info!(version = %app.package_info().version, "Lamina PDF started");
    // A cold start's PDFs (Windows, Linux, a macOS terminal), after the restored tabs, in the
    // first window: queued until its webview takes them.
    let first = labels.first().map_or(MAIN_WINDOW, String::as_str);
    let cwd = std::env::current_dir().unwrap_or_default();
    let args: Vec<OsString> = std::env::args_os().collect();
    open::route_open(app.handle(), first, open::paths_from_args(&args, &cwd));
    Ok(())
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
    #[cfg(feature = "e2e")]
    e2e::start_clock();
    // The test commands and WebdriverIO's plugins exist only in e2e builds; generate_handler!
    // takes no cfg per command.
    #[cfg(not(feature = "e2e"))]
    let builder = tauri::Builder::default().invoke_handler(tauri::generate_handler![
        commands::app_info,
        commands::open_dialog,
        commands::page_info,
        commands::close_document,
        commands::unlock_document,
        commands::cancel_tiles,
        commands::recent_list,
        commands::recent_open,
        commands::recent_remove,
        commands::recent_clear,
        commands::preferences,
        commands::set_preferences,
        commands::new_window,
        commands::move_tab,
        commands::window_list,
        commands::set_window_title,
        commands::take_pending_tabs,
        session::app::session_tabs,
        session::app::take_session_tabs,
        session::app::session_open,
        commands::menu_state,
        commands::close_window,
        commands::quit,
        commands::save_copy,
        open::take_pending_opens,
    ]);
    #[cfg(feature = "e2e")]
    let builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::open_dialog,
            commands::page_info,
            commands::close_document,
            commands::unlock_document,
            commands::cancel_tiles,
            commands::recent_list,
            commands::recent_open,
            commands::recent_remove,
            commands::recent_clear,
            commands::preferences,
            commands::set_preferences,
            commands::new_window,
            commands::move_tab,
            commands::window_list,
            commands::set_window_title,
            commands::take_pending_tabs,
            session::app::session_tabs,
            session::app::take_session_tabs,
            session::app::session_open,
            commands::menu_state,
            commands::close_window,
            commands::quit,
            commands::save_copy,
            open::take_pending_opens,
            commands::debug_kill_engine,
            e2e::e2e_queue_pick,
            e2e::e2e_interactive,
            e2e::e2e_focus,
            e2e::e2e_menu_click,
            e2e::e2e_menu_snapshot,
        ])
        .manage(e2e::QueuedPick::default());
    // First of all plugins: a second launch hands its paths to the running app and exits before
    // anything else starts (the plugin's own rule). Its callback queues them like any OS request.
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            second_launch(app, &args, &cwd);
        }))
        .manage(open::PendingOpens::default());
    let builder = builder.plugin(
        tauri_plugin_log::Builder::new()
            // Stdout and the OS log folder. Document paths are logged at `debug`, never `info`.
            .level(tauri_plugin_log::log::LevelFilter::Info)
            .build(),
    );
    // WebdriverIO's test API (scripts, logs) and the WebDriver server inside the app, after
    // the log plugin: tauri-plugin-wdio installs its own logger when none is set yet.
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    let app = builder
        // Tiles for the viewer (ADR 0008 §4). The window is the webview that asked, never part of
        // the address, so a window reads only its own documents; the engine's wait and the WebP
        // encoding run off the main thread.
        .register_asynchronous_uri_scheme_protocol(TILE_SCHEME, |ctx, request, responder| {
            let window = ctx.webview_label().to_owned();
            let engines = Arc::clone(&ctx.app_handle().state::<Arc<Engines>>());
            let uri = request.uri().clone();
            let is_get = request.method() == tauri::http::Method::GET;
            tauri::async_runtime::spawn_blocking(move || {
                let answer = if is_get {
                    engine::parse_tile(uri.path(), uri.query())
                        .map_err(DesktopError::from)
                        .and_then(|query| engines.render_tile(&window, &query).map(|tile| (tile, query.encoding)))
                } else {
                    Err(DesktopError::from(engine::TileUrlError::NotATile))
                };
                responder.respond(engine::tile_response(answer));
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Each window's size, position and maximised state, kept by its label and put back when a
        // window of that label opens (D4d); a position on a monitor that went away isn't restored.
        // Rust's own: the webview is granted none of its commands.
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(session::app::WINDOW_STATE)
                .build(),
        )
        .setup(setup)
        .on_window_event(on_window_event)
        .on_menu_event(|app, event| menu::on_menu_event(app, &event))
        .build(tauri::generate_context!())?;
    app.run(|app, event| on_run_event(app, &event));
    Ok(())
}

/// A second launch's files, which the single-instance plugin forwards: they open in the frontmost
/// window; a launch with no file still brings that window to the front.
fn second_launch(app: &tauri::AppHandle, args: &[String], cwd: &str) {
    let paths = open::paths_from_args(args, std::path::Path::new(cwd));
    match windows::target_window(app) {
        Ok(window) => {
            if paths.is_empty() {
                open::bring_forward(app, &window);
            }
            open::route_open(app, &window, paths);
        }
        Err(error) => tracing::warn!("no window for a second launch's files: {error}"),
    }
}

fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    match event {
        // A closed window's documents close, their passwords are wiped, its engine stops, and
        // their files are no longer watched.
        WindowEvent::Destroyed => {
            session::app::window_closed(window.app_handle(), window.label());
            window.state::<Arc<Engines>>().close_window(window.label());
            window.state::<watch::Watcher>().unwatch_window(window.label());
            window.state::<windows::Windows>().closed(window.label());
            window.state::<open::PendingOpens>().closed(window.label());
            menu::closed(window.app_handle(), window.label());
            windows::changed(window.app_handle());
        }
        // The OS's open requests go to the window focused last.
        // The menus show the focused window's commands.
        WindowEvent::Focused(true) => {
            window.state::<windows::Windows>().focused(window.label());
            menu::refresh(window.app_handle());
        }
        // Files dragged over or dropped on the window: Rust opens them, never the webview.
        WindowEvent::DragDrop(drag) => open::on_drag_drop(window, drag),
        _ => {}
    }
}

fn on_run_event(app: &tauri::AppHandle, event: &RunEvent) {
    match event {
        RunEvent::Exit => {
            session::app::flush(app);
            app.state::<Arc<Engines>>().shutdown();
        }
        // macOS hands Finder's, the Dock's and `open`'s files to the running app, cold or warm,
        // with or without a window.
        #[cfg(target_os = "macos")]
        RunEvent::Opened { urls } => match windows::target_window(app) {
            Ok(window) => open::route_open(app, &window, open::paths_from_urls(urls)),
            Err(error) => tracing::warn!("no window for the files macOS opened: {error}"),
        },
        // A macOS app keeps running when its last window closes; quitting is the user's (⌘Q).
        #[cfg(target_os = "macos")]
        RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
        // The app quits (⌘Q, or the last window closed outside macOS): the windows that close now
        // stay in the session, which is written before they go.
        RunEvent::ExitRequested { .. } => session::app::quitting(app),
        // The Dock icon clicked with no window open: a new, empty one.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => {
            if let Err(error) = windows::new_window(app) {
                tracing::warn!("could not open a window from the Dock: {error}");
            }
        }
        _ => {}
    }
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
