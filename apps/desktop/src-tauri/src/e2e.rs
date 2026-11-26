//! End-to-end test builds only (feature `e2e`): what the `WebdriverIO` scenarios need that a
//! user does by hand. A native file dialog can't be driven, so a scenario queues a fixture with
//! `e2e_queue_pick` and clicks Choose a file, and `open_dialog` takes the queued path instead of
//! showing the dialog. Only files under the repository's `fixtures/` or `target/fixtures/` may be
//! queued. `e2e_interactive` reports the cold start: the time from `run()` to the empty state
//! accepting input. `e2e_menu_click` and `e2e_menu_snapshot` choose a native menu item and read
//! the menus, which `WebDriver` can't reach; `e2e_focus` brings a window to the front. Release
//! builds have none of this; CI checks their binary (ADR 0008 §8).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use tauri::Manager as _;

use crate::DesktopError;
use crate::tab_drag::{Frame, Point};

/// When `run()` began: the start of the cold start `e2e_interactive` reports.
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Starts the cold start's clock and the startup marks' (`startup.rs`); `run()` calls it first.
pub(crate) fn start_clock() {
    STARTED.get_or_init(Instant::now);
    crate::startup::start();
}

/// The frontend's empty state accepts input: prints and returns the milliseconds since `run()`
/// began (`e2e/cold-start.ts` reads the line).
#[tauri::command]
pub(crate) fn e2e_interactive() -> u128 {
    let ms = STARTED.get().map_or(0, |started| started.elapsed().as_millis());
    println!("e2e: interactive {ms} ms");
    ms
}

/// Brings this window to the front, as a click on it would. Once another window closes, macOS
/// can leave the one under it in the background, where `WebKit` slows its work; and the menus
/// act on the focused window. Async, as the commands that focus or make windows are: on Windows
/// a synchronous command runs on the main thread, which those wait for.
#[tauri::command]
pub(crate) async fn e2e_focus(window: tauri::WebviewWindow) -> Result<(), DesktopError> {
    focus(&window)
}

fn focus(window: &tauri::WebviewWindow) -> Result<(), DesktopError> {
    window.set_focus()?;
    // The OS reports the focus later; the menus need it now.
    window.state::<crate::windows::Windows>().focused(window.label());
    crate::menu::refresh(window.app_handle());
    Ok(())
}

/// Chooses the native menu item `id` in this window, as a click on its menu would
/// (`cmd:view.zoom-in`, `recent:<id>`, `window:doc-1`): `WebDriver` can't reach the OS's menus.
#[tauri::command]
pub(crate) async fn e2e_menu_click(window: tauri::WebviewWindow, id: String) -> Result<(), DesktopError> {
    focus(&window)?;
    crate::menu::run(window.app_handle(), crate::menu::action(&id));
    Ok(())
}

/// The native menus as they show now, for the focused window, one item a line (`menu::render`):
/// titles, keys, enabled and ticked. Reading them changes nothing, focus included.
#[tauri::command]
#[expect(clippy::needless_pass_by_value, reason = "Tauri hands a command its app by value")]
pub(crate) fn e2e_menu_snapshot(app: tauri::AppHandle) -> String {
    crate::menu::snapshot(&app)
}

/// The path the next `open_dialog` opens instead of asking, per app.
#[derive(Debug, Default)]
pub(crate) struct QueuedPick(Mutex<Option<PathBuf>>);

impl QueuedPick {
    /// Queues `path`, replacing any path still queued.
    pub(crate) fn set(&self, path: PathBuf) {
        *self.slot() = Some(path);
    }

    /// Takes the queued path, leaving none.
    pub(crate) fn take(&self) -> Option<PathBuf> {
        self.slot().take()
    }

    /// The slot, also after a panic elsewhere poisoned the lock: a path is all it holds.
    fn slot(&self) -> std::sync::MutexGuard<'_, Option<PathBuf>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The folders a scenario may open files from: the repository's fixtures, committed and generated.
fn fixture_roots() -> Vec<PathBuf> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..");
    [repo.join("fixtures"), repo.join("target").join("fixtures")]
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect()
}

/// `path` resolved, when it is a file under one of `roots`.
fn fixture_path(path: &Path, roots: &[PathBuf]) -> Option<PathBuf> {
    let resolved = path.canonicalize().ok()?;
    (resolved.is_file() && roots.iter().any(|root| resolved.starts_with(root))).then_some(resolved)
}

/// Queues `path` for the next Choose a file.
///
/// # Errors
/// [`DesktopError::Internal`] when `path` isn't a file under the repository's fixtures.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its state and arguments by value"
)]
pub(crate) fn e2e_queue_pick(queue: tauri::State<'_, QueuedPick>, path: String) -> Result<(), DesktopError> {
    let resolved = fixture_path(Path::new(&path), &fixture_roots())
        .ok_or_else(|| DesktopError::Internal("e2e_queue_pick: not a fixture".to_owned()))?;
    queue.set(resolved);
    Ok(())
}

/// The path the next Save as writes instead of asking, per app.
#[derive(Debug, Default)]
pub(crate) struct QueuedSaveAs(QueuedPick);

impl QueuedSaveAs {
    /// Takes the queued path, leaving none.
    pub(crate) fn take(&self) -> Option<PathBuf> {
        self.0.take()
    }
}

/// Queues `path` for the next Save as: a file to write in a folder under the repository's
/// fixtures, which needn't exist yet.
///
/// # Errors
/// [`DesktopError::Internal`] when `path`'s folder isn't under the repository's fixtures.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its state and arguments by value"
)]
pub(crate) fn e2e_queue_save_as(queue: tauri::State<'_, QueuedSaveAs>, path: String) -> Result<(), DesktopError> {
    let path = Path::new(&path);
    let refused = || DesktopError::Internal("e2e_queue_save_as: not in a fixtures folder".to_owned());
    let folder = path
        .parent()
        .and_then(|folder| folder.canonicalize().ok())
        .ok_or_else(refused)?;
    let name = path.file_name().ok_or_else(refused)?;
    if !fixture_roots().iter().any(|root| folder.starts_with(root)) {
        return Err(refused());
    }
    queue.0.set(folder.join(name));
    Ok(())
}

/// The folder the next Choose a folder takes instead of asking, per app.
#[derive(Debug, Default)]
pub(crate) struct QueuedFolder(QueuedPick);

impl QueuedFolder {
    /// Takes the queued folder, leaving none.
    pub(crate) fn take(&self) -> Option<PathBuf> {
        self.0.take()
    }
}

/// Queues `path` for the next folder dialog: a folder under the repository's fixtures.
///
/// # Errors
/// [`DesktopError::Internal`] when `path` isn't a folder under the repository's fixtures.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its state and arguments by value"
)]
pub(crate) fn e2e_queue_folder(queue: tauri::State<'_, QueuedFolder>, path: String) -> Result<(), DesktopError> {
    let resolved = Path::new(&path)
        .canonicalize()
        .ok()
        .filter(|folder| folder.is_dir() && fixture_roots().iter().any(|root| folder.starts_with(root)))
        .ok_or_else(|| DesktopError::Internal("e2e_queue_folder: not a folder under the fixtures".to_owned()))?;
    queue.0.set(resolved);
    Ok(())
}

/// How far inside a window's edge `visible_point` tries its corners, in points: clear of borders.
const CORNER_INSET: f64 = 24.0;
/// How far right of every window `outside` puts the cursor, in points.
const OUTSIDE_GAP: f64 = 400.0;
/// How far below the first window's top `outside` puts the cursor, in points.
const OUTSIDE_DROP: f64 = 100.0;

/// Where `e2e_place_cursor` put the cursor that a tab's drag reads, if anywhere: `WebDriver` moves
/// the pointer inside a page only, never the OS's cursor.
static PLACED_CURSOR: Mutex<Option<Point>> = Mutex::new(None);

/// The cursor a tab's drag reads instead of the OS's, once a test placed one.
pub(crate) fn placed_cursor() -> Option<Point> {
    *PLACED_CURSOR.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A point of window `label` that no window in front of it covers: its middle, else a point near
/// one of its corners. Windows may overlap, and a drop where another window is in front lands on
/// that one.
fn visible_point(label: &str, frames: &[(String, Frame)]) -> Option<Point> {
    let (_, frame) = frames.iter().find(|(name, _)| name == label)?;
    [(0.5, 0.5), (0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        .into_iter()
        .map(|(across, down)| Point {
            x: frame.x + CORNER_INSET + across * (frame.width - 2.0 * CORNER_INSET),
            y: frame.y + CORNER_INSET + down * (frame.height - 2.0 * CORNER_INSET),
        })
        .find(|point| crate::tab_drag::window_at(*point, frames) == Some(label))
}

/// Places the cursor a tab's drag reads (`src/tab_drag.rs`): `window:<label>` puts it in the
/// middle of that window, `outside` to the right of every window, `os` gives back the OS's own.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
pub(crate) fn e2e_place_cursor(app: tauri::AppHandle, at: String) -> Result<(), DesktopError> {
    let frames = crate::tab_drag::window_frames(&app);
    let point = match at.as_str() {
        "os" => None,
        "outside" => {
            let right = frames
                .iter()
                .map(|(_, frame)| frame.x + frame.width)
                .fold(0.0, f64::max);
            let top = frames.first().map_or(0.0, |(_, frame)| frame.y);
            Some(Point {
                x: right + OUTSIDE_GAP,
                y: top + OUTSIDE_DROP,
            })
        }
        _ => {
            let label = at
                .strip_prefix("window:")
                .ok_or_else(|| DesktopError::Internal(format!("no place {at}")))?;
            Some(
                visible_point(label, &frames)
                    .ok_or_else(|| DesktopError::Internal(format!("{label} isn't showing")))?,
            )
        }
    };
    *PLACED_CURSOR.lock().unwrap_or_else(PoisonError::into_inner) = point;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{QueuedPick, fixture_path, fixture_roots};
    use std::path::Path;

    #[test]
    fn a_fixture_should_be_allowed() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/golden/hello.pdf");
        assert!(fixture_path(&fixture, &fixture_roots()).is_some());
    }

    #[test]
    fn a_file_outside_the_fixtures_should_be_refused() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(fixture_path(&manifest, &fixture_roots()).is_none());
    }

    #[test]
    fn a_path_climbing_out_of_the_fixtures_should_be_refused() {
        let climbing = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/../Cargo.toml");
        assert!(fixture_path(&climbing, &fixture_roots()).is_none());
    }

    #[test]
    fn take_should_empty_the_queue() {
        let queue = QueuedPick::default();
        queue.set("a.pdf".into());
        assert!(queue.take().is_some());
        assert!(queue.take().is_none());
    }
}
