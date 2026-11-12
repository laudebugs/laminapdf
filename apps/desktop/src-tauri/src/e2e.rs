//! End-to-end test builds only (feature `e2e`): what the `WebdriverIO` scenarios need that a
//! user does by hand. A native file dialog can't be driven, so a scenario queues a fixture with
//! `e2e_queue_pick` and clicks Choose a file, and `open_dialog` takes the queued path instead of
//! showing the dialog. Only files under the repository's `fixtures/` or `target/fixtures/` may be
//! queued. `e2e_interactive` reports the cold start: the time from `run()` to the empty state
//! accepting input. Release builds have none of this; CI checks their binary (ADR 0008 §8).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use crate::DesktopError;

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
