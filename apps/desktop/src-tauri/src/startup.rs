//! Cold-start timing (D2g), in `e2e` builds only: when the process reached `run`, `setup` began,
//! the main window was built and the webview's first command arrived. The first command logs them
//! once, so a script that launches the app reads them from its output (`tools/desktop/cold_start.py`,
//! ADR 0008 §4).

use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// The process's marks; each is set once.
struct Marks {
    start: Instant,
    /// Milliseconds since the Unix epoch at `start`, so a launcher can add launch → `run`.
    start_unix_ms: u128,
    setup: OnceLock<Instant>,
    window: OnceLock<Instant>,
    first_command: OnceLock<Instant>,
}

impl Marks {
    /// Milliseconds from the start of `run` to `mark`, once it is reached.
    fn since_start(&self, mark: &OnceLock<Instant>) -> Option<f64> {
        mark.get()
            .map(|at| at.duration_since(self.start).as_secs_f64() * 1000.0)
    }
}

static MARKS: OnceLock<Marks> = OnceLock::new();

/// Sets `mark` to now, unless it is set; reports whether this call set it.
fn reach(mark: impl Fn(&Marks) -> &OnceLock<Instant>) -> bool {
    MARKS.get().is_some_and(|marks| mark(marks).set(Instant::now()).is_ok())
}

/// The start of `run`: call it first.
pub(crate) fn start() {
    let start_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    // A second call keeps the first start.
    let _ignored = MARKS.set(Marks {
        start: Instant::now(),
        start_unix_ms,
        setup: OnceLock::new(),
        window: OnceLock::new(),
        first_command: OnceLock::new(),
    });
}

/// `setup` has begun.
pub(crate) fn setup() {
    reach(|marks| &marks.setup);
}

/// The main window is built.
pub(crate) fn window() {
    reach(|marks| &marks.window);
}

/// A command arrived from the webview; the first one logs every mark.
pub(crate) fn command() {
    if !reach(|marks| &marks.first_command) {
        return;
    }
    let Some(marks) = MARKS.get() else { return };
    tracing::info!(
        target: "laminapdf_desktop::startup",
        start_unix_ms = %marks.start_unix_ms,
        setup_ms = ?marks.since_start(&marks.setup),
        window_ms = ?marks.since_start(&marks.window),
        first_command_ms = ?marks.since_start(&marks.first_command),
        "startup timings"
    );
}
