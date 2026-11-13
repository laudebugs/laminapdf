//! Dragging a tab out of its window or onto another one. A webview can't read a drop that lands
//! in another window, so Rust follows the drag instead: the tab's window calls [`start`] when the
//! drag begins and [`end`] when it ends. In between, a thread reads the OS's cursor and tells the
//! window under it that a tab may drop there ([`DRAG_OVER_EVENT`]). At the end Rust looks where the
//! cursor is: over another window, the tab moves there; outside every window, a new window opens
//! under the cursor with the tab; over its own window, nothing happens (the strip's own drop
//! reorders). Both moves go through [`crate::windows::move_tab`], as the tab menu's do.
//!
//! Under Wayland the compositor keeps windows' positions to itself, so Rust relies on the webview's
//! report of whether the drag left its window: if it did, a new window opens wherever the
//! compositor puts it, and no other window can be found under the cursor.
//!
//! The end of a drag can't tell a drop from a cancel (Escape): a cancelled drag whose cursor is
//! outside its window moves the tab too. Other apps' windows are invisible here, so a drop on one
//! that covers a Lamina PDF window moves the tab to the covered window.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use laminapdf_desktop_protocol::DocId;
use tauri::{AppHandle, Emitter as _, EventTarget, LogicalPosition, Manager as _, PhysicalPosition, Position};

use crate::DesktopError;
use crate::windows::{MoveTo, View, Windows};

/// Whether a dragged tab is over this window (`true`) or has left it (`false`). The event's name is
/// repeated in `src/tabs/window-tabs.ts`.
pub(crate) const DRAG_OVER_EVENT: &str = "tab://drag-over";

/// How often the cursor is read while a tab is dragged.
const POLL: Duration = Duration::from_millis(50);
/// A drag whose end never came (its window closed mid-drag) stops being followed after this.
const LONGEST: Duration = Duration::from_secs(120);
/// Where the cursor sits in a window opened by a drop, from its top-left corner, in logical
/// pixels: over the start of the tab strip, as if the tab were still held.
const GRIP: Point = Point { x: 96.0, y: 12.0 };

/// A point on the screen, in the space [`window_frames`] and [`cursor`] share.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// A window's frame on the screen (title bar and borders included).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl Frame {
    fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.x + self.width && point.y >= self.y && point.y < self.y + self.height
    }
}

/// What a drag's end does with the tab.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Target {
    /// Nothing: it ended over its own window.
    Stay,
    /// It moves to this window.
    Window(String),
    /// It moves to a new window, opened with the cursor at `GRIP` when the cursor is known.
    NewWindow(Option<Point>),
}

/// The window under `cursor`: `frames` go front to back, so the first that holds it is on top.
pub(crate) fn window_at(cursor: Point, frames: &[(String, Frame)]) -> Option<&str> {
    frames
        .iter()
        .find(|(_, frame)| frame.contains(cursor))
        .map(|(label, _)| label.as_str())
}

/// Where a tab dragged from `from` goes when the drag ends. `cursor` is `None` when the OS won't
/// say; `outside` is the webview's account of whether the pointer left `from`, used only then.
pub(crate) fn decide(from: &str, cursor: Option<Point>, outside: bool, frames: &[(String, Frame)]) -> Target {
    let Some(cursor) = cursor else {
        return if outside { Target::NewWindow(None) } else { Target::Stay };
    };
    match window_at(cursor, frames) {
        Some(label) if label == from => Target::Stay,
        Some(label) => Target::Window(label.to_owned()),
        // Outside every window whose frame is known. With none known (Wayland), the cursor's
        // position alone can't say it left its window.
        None if frames.is_empty() && !outside => Target::Stay,
        None => Target::NewWindow(Some(cursor)),
    }
}

/// The drag in progress, if any: one at a time, as there is one pointer.
#[derive(Debug, Default)]
pub(crate) struct TabDrag(Mutex<Option<Following>>);

#[derive(Debug)]
struct Following {
    from: String,
    doc: DocId,
    stop: Arc<AtomicBool>,
}

impl TabDrag {
    fn state(&self) -> MutexGuard<'_, Option<Following>> {
        // One small record: nothing a panic could leave half-done.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Follows a new drag, ending the one before (whose end never came). Returns its stop flag.
    fn begin(&self, from: &str, doc: DocId) -> Arc<AtomicBool> {
        let stop = Arc::new(AtomicBool::new(false));
        let following = Following {
            from: from.to_owned(),
            doc,
            stop: Arc::clone(&stop),
        };
        if let Some(previous) = self.state().replace(following) {
            previous.stop.store(true, Ordering::Relaxed);
        }
        stop
    }

    /// Stops following the drag of `doc` from `from`, if it is the one followed.
    fn finish(&self, from: &str, doc: DocId) {
        let mut state = self.state();
        if state.as_ref().is_some_and(|f| f.from == from && f.doc == doc)
            && let Some(following) = state.take()
        {
            following.stop.store(true, Ordering::Relaxed);
        }
    }
}

/// Where a window opened by a drop at `cursor` puts its corner: up and left of the cursor by
/// `GRIP`, in logical pixels times `scale` (1 where `cursor` is in points). The corner may be
/// negative, on a screen left of or above the main one.
pub(crate) fn corner(cursor: Point, scale: f64) -> Point {
    Point {
        x: cursor.x - GRIP.x * scale,
        y: cursor.y - GRIP.y * scale,
    }
}

/// Whether the OS says where windows and the cursor are: everywhere but under a Wayland
/// compositor (GTK runs on Wayland when `WAYLAND_DISPLAY` is set, unless `GDK_BACKEND` asks for X11).
fn positions_known() -> bool {
    !cfg!(target_os = "linux")
        || std::env::var_os("WAYLAND_DISPLAY").is_none()
        || std::env::var("GDK_BACKEND").is_ok_and(|backend| backend.starts_with("x11"))
}

/// The factor that takes a frame or the cursor into the space they share: on macOS, points (each
/// reported scaled by its own screen's factor, which `scale` reads); elsewhere physical pixels,
/// one space across screens, so 1.
fn to_shared_space(scale: impl FnOnce() -> Option<f64>) -> Option<f64> {
    if cfg!(target_os = "macos") { scale() } else { Some(1.0) }
}

/// The open, visible windows' frames, front to back (the window focused last first). On macOS
/// in points, on the other systems in physical pixels: the same space as [`cursor`]'s.
fn window_frames(app: &AppHandle) -> Vec<(String, Frame)> {
    if !positions_known() {
        return Vec::new();
    }
    let mut labels = app.state::<Windows>().front_to_back();
    for label in app.webview_windows().into_keys() {
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    labels
        .into_iter()
        .filter_map(|label| {
            let window = app.get_webview_window(&label)?;
            if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
                return None;
            }
            let position = window.outer_position().ok()?;
            let size = window.outer_size().ok()?;
            let scale = to_shared_space(|| window.scale_factor().ok())?;
            let frame = Frame {
                x: f64::from(position.x) / scale,
                y: f64::from(position.y) / scale,
                width: f64::from(size.width) / scale,
                height: f64::from(size.height) / scale,
            };
            Some((label, frame))
        })
        .collect()
}

/// Where the cursor is, if the OS says. macOS reports it scaled by the main screen's factor.
fn cursor(app: &AppHandle) -> Option<Point> {
    if !positions_known() {
        return None;
    }
    let position = app.cursor_position().ok()?;
    let scale = to_shared_space(|| Some(app.primary_monitor().ok().flatten()?.scale_factor()))?;
    Some(Point {
        x: position.x / scale,
        y: position.y / scale,
    })
}

/// The corner of a window opened by a drop at `cursor`: in points on macOS; elsewhere in physical
/// pixels, so a screen with its own scale factor places it right, with `GRIP` scaled by the factor
/// of the screen under the cursor.
fn new_window_corner(app: &AppHandle, cursor: Point) -> Position {
    if cfg!(target_os = "macos") {
        let at = corner(cursor, 1.0);
        return Position::Logical(LogicalPosition::new(at.x, at.y));
    }
    let scale = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .map_or(1.0, |monitor| monitor.scale_factor());
    let at = corner(cursor, scale);
    // Whole pixels: the cast saturates far off any screen.
    #[expect(clippy::cast_possible_truncation, reason = "screen coordinates fit an i32")]
    Position::Physical(PhysicalPosition::new(at.x.round() as i32, at.y.round() as i32))
}

/// A tab of `from`, its document `doc`, began to be dragged: the window under the cursor is told
/// so until the drag ends.
pub(crate) fn start(app: &AppHandle, from: &str, doc: DocId) {
    let stop = app.state::<TabDrag>().begin(from, doc);
    let app = app.clone();
    let from = from.to_owned();
    let follow = move || {
        let began = Instant::now();
        let mut over: Option<String> = None;
        while !stop.load(Ordering::Relaxed) && began.elapsed() < LONGEST {
            let under = cursor(&app).and_then(|at| {
                let frames = window_frames(&app);
                window_at(at, &frames).filter(|label| *label != from).map(str::to_owned)
            });
            if under != over {
                tell(&app, over.as_deref(), false);
                tell(&app, under.as_deref(), true);
                over = under;
            }
            std::thread::sleep(POLL);
        }
        tell(&app, over.as_deref(), false);
    };
    if let Err(error) = std::thread::Builder::new().name("tab-drag".into()).spawn(follow) {
        tracing::warn!("could not follow a dragged tab: {error}");
    }
}

fn tell(app: &AppHandle, label: Option<&str>, over: bool) {
    let Some(label) = label else { return };
    if let Err(error) = app.emit_to(EventTarget::webview_window(label), DRAG_OVER_EVENT, over) {
        tracing::warn!("could not tell a window a tab is dragged over it: {error}");
    }
}

/// The drag of `from`'s tab of `doc` ended, outside `from` by the webview's account or not: the
/// tab moves where the cursor is, with `view`. Returns the label of the window it went to, or
/// `None` when it stays.
pub(crate) fn end(
    app: &AppHandle,
    from: &str,
    doc: DocId,
    view: View,
    outside: bool,
) -> Result<Option<String>, DesktopError> {
    app.state::<TabDrag>().finish(from, doc);
    let cursor = cursor(app);
    let frames = window_frames(app);
    let to = match decide(from, cursor, outside, &frames) {
        Target::Stay => return Ok(None),
        Target::Window(label) => crate::windows::move_tab(app, from, doc, &MoveTo::Window(label), view, None)?,
        Target::NewWindow(at) => {
            let corner = at.map(|at| new_window_corner(app, at));
            crate::windows::move_tab(app, from, doc, &MoveTo::NewWindow, view, corner)?
        }
    };
    Ok(Some(to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: f64, y: f64, width: f64, height: f64) -> Frame {
        Frame { x, y, width, height }
    }

    fn at(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    /// `main` at the left, `doc-1` at the right, and `doc-2` in front of both where they meet.
    fn frames() -> Vec<(String, Frame)> {
        vec![
            ("doc-2".into(), frame(400.0, 100.0, 300.0, 200.0)),
            ("main".into(), frame(0.0, 0.0, 600.0, 400.0)),
            ("doc-1".into(), frame(600.0, 0.0, 600.0, 400.0)),
        ]
    }

    #[test]
    fn the_window_in_front_takes_the_cursor_where_windows_overlap() {
        let frames = frames();
        let window = |x, y| window_at(Point { x, y }, &frames);
        assert_eq!(window(450.0, 150.0), Some("doc-2"));
        assert_eq!(window(100.0, 100.0), Some("main"));
        assert_eq!(window(650.0, 350.0), Some("doc-1"));
        assert_eq!(window(1300.0, 100.0), None);
    }

    #[test]
    fn a_frame_holds_its_top_left_edge_but_not_its_bottom_right_one() {
        let frames = vec![("main".to_owned(), frame(10.0, 10.0, 100.0, 50.0))];
        assert_eq!(window_at(Point { x: 10.0, y: 10.0 }, &frames), Some("main"));
        assert_eq!(window_at(Point { x: 110.0, y: 30.0 }, &frames), None);
        assert_eq!(window_at(Point { x: 50.0, y: 60.0 }, &frames), None);
    }

    #[test]
    fn a_drop_on_another_window_moves_the_tab_there() {
        assert_eq!(
            decide("main", Some(at(900.0, 20.0)), true, &frames()),
            Target::Window("doc-1".into())
        );
    }

    #[test]
    fn a_drop_on_its_own_window_leaves_the_tab_alone() {
        assert_eq!(decide("main", Some(at(100.0, 20.0)), false, &frames()), Target::Stay);
        // The cursor wins over the webview's account, which can miss a re-entry.
        assert_eq!(decide("main", Some(at(100.0, 20.0)), true, &frames()), Target::Stay);
    }

    #[test]
    fn a_drop_outside_every_window_opens_one_there() {
        assert_eq!(
            decide("main", Some(at(1300.0, 500.0)), false, &frames()),
            Target::NewWindow(Some(at(1300.0, 500.0)))
        );
    }

    #[test]
    fn without_the_cursor_the_webviews_account_decides() {
        assert_eq!(decide("main", None, true, &frames()), Target::NewWindow(None));
        assert_eq!(decide("main", None, false, &frames()), Target::Stay);
    }

    #[test]
    fn without_window_frames_the_cursor_alone_opens_no_window() {
        assert_eq!(decide("main", Some(at(5.0, 5.0)), false, &[]), Target::Stay);
        assert_eq!(
            decide("main", Some(at(5.0, 5.0)), true, &[]),
            Target::NewWindow(Some(at(5.0, 5.0)))
        );
    }

    #[test]
    fn a_new_window_puts_the_grip_under_the_cursor_on_any_screen() {
        assert_eq!(corner(at(500.0, 300.0), 1.0), at(404.0, 288.0));
        assert_eq!(corner(at(500.0, 300.0), 2.0), at(308.0, 276.0));
        // Left of and above the main screen, coordinates go below zero and stay there.
        assert_eq!(corner(at(-500.0, -20.0), 1.0), at(-596.0, -32.0));
    }

    #[test]
    fn a_new_drag_stops_following_the_one_whose_end_never_came() {
        let drag = TabDrag::default();
        let first = drag.begin("main", DocId(1));
        let second = drag.begin("main", DocId(2));
        assert!(first.load(Ordering::Relaxed));
        // The first drag's late end leaves the second alone.
        drag.finish("main", DocId(1));
        assert!(!second.load(Ordering::Relaxed));
        drag.finish("main", DocId(2));
        assert!(second.load(Ordering::Relaxed));
    }
}
