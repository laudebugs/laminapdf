//! The app's windows: each shows the documents of its tabs, and a tab can move to a
//! new window or another one. Rust makes every window, with the labels the capability allows
//! (`main` first, then `doc-<n>`), keeps track of the frontmost one, which the OS's open requests
//! go to, and hands a moved tab to its new window once that window's webview is ready.
//!
//! The tabs themselves (their order, the active one, each one's view) live in the webview: Rust
//! knows only which window owns each document (`DocumentRegistry`).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use laminapdf_desktop_protocol::DocId;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _, EventTarget, Manager as _, Position, WebviewUrl, WebviewWindowBuilder};

use crate::engine::{DocumentMeta, Engines};
use crate::{DesktopError, MAIN_WINDOW};

/// A tab moved to this window: its document, opened in this window's engine, and the view it
/// had. The event's name is repeated in `src/tabs/window-tabs.ts`.
pub(crate) const ARRIVED_EVENT: &str = "tab://arrived";
/// Show the tab of this document (the same file was opened again).
pub(crate) const ACTIVATE_EVENT: &str = "tab://activate";
/// A window opened, closed or changed its title: every window reads `window_list` again.
pub(crate) const CHANGED_EVENT: &str = "windows://changed";

/// The view a tab had (page, offset, zoom, rotation), as the webview saved it: Rust passes it
/// on to the tab's new window and never reads it.
pub(crate) type View = serde_json::Value;

/// What [`ARRIVED_EVENT`] carries.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Arrived {
    pub(crate) document: DocumentMeta,
    pub(crate) view: View,
}

/// Another window, for "Move tab to window ▸".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WindowEntry {
    pub(crate) label: String,
    pub(crate) title: String,
}

/// Where a moved tab goes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", tag = "to", content = "label")]
pub(crate) enum MoveTo {
    /// A new window, made for it.
    NewWindow,
    /// An open window, by label.
    Window(String),
}

/// The windows' shared state.
#[derive(Debug, Default)]
pub(crate) struct Windows(Mutex<State>);

#[derive(Debug, Default)]
struct State {
    /// The last number given to a `doc-<n>` window.
    last: u32,
    /// The windows by when they were last focused, the last one first: open requests go to the
    /// first, and a dragged tab's drop finds the window in front where windows overlap.
    focus: Vec<String>,
    /// Tabs moved to windows whose webview hasn't taken them yet.
    arriving: BTreeMap<String, Vec<Arrived>>,
    /// Windows whose webview has taken its arrivals: later ones are sent at once.
    ready: Vec<String>,
}

impl Windows {
    fn state(&self) -> MutexGuard<'_, State> {
        // Labels and queues: nothing a panic could leave half-done.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A label for a new window: `doc-1`, `doc-2`, … never reused.
    pub(crate) fn next_label(&self) -> String {
        let mut state = self.state();
        state.last += 1;
        format!("doc-{}", state.last)
    }

    /// `label` came to the front.
    pub(crate) fn focused(&self, label: &str) {
        let mut state = self.state();
        state.focus.retain(|focused| focused != label);
        state.focus.insert(0, label.to_owned());
    }

    /// `label` closed: it is no longer frontmost, and anything waiting for it is dropped.
    pub(crate) fn closed(&self, label: &str) {
        let mut state = self.state();
        state.focus.retain(|focused| focused != label);
        state.arriving.remove(label);
        state.ready.retain(|ready| ready != label);
    }

    /// The open window focused last; `None` before any was focused or once all closed.
    pub(crate) fn frontmost(&self) -> Option<String> {
        self.state().focus.first().cloned()
    }

    /// The windows focused so far, the one focused last first.
    pub(crate) fn front_to_back(&self) -> Vec<String> {
        self.state().focus.clone()
    }

    /// Queues `arrived` for `label` until its webview takes it (`None`), or hands it back to send
    /// now when that webview is ready. One lock covers the check and the push, as for open requests.
    fn offer(&self, label: &str, arrived: Arrived) -> Option<Arrived> {
        let mut state = self.state();
        if state.ready.iter().any(|ready| ready == label) {
            return Some(arrived);
        }
        state.arriving.entry(label.to_owned()).or_default().push(arrived);
        None
    }

    /// What arrived for `label` before its webview was ready; from now on arrivals go at once.
    fn take(&self, label: &str) -> Vec<Arrived> {
        let mut state = self.state();
        if !state.ready.iter().any(|ready| ready == label) {
            state.ready.push(label.to_owned());
        }
        state.arriving.remove(label).unwrap_or_default()
    }
}

/// The window the OS's open requests go to: the frontmost one, else the first open window, else a new
/// one (macOS keeps running with no windows).
pub(crate) fn target_window(app: &AppHandle) -> Result<String, DesktopError> {
    let windows = app.state::<Windows>();
    if let Some(label) = windows
        .frontmost()
        .filter(|label| app.get_webview_window(label).is_some())
    {
        return Ok(label);
    }
    if let Some(label) = app
        .webview_windows()
        .into_keys()
        .min_by_key(|label| window_order(label))
    {
        return Ok(label);
    }
    new_window(app, None)
}

/// Makes a window labelled `label`: the app's page, a title, a size that fits the screen, and its
/// top-left corner at `corner` when given.
pub(crate) fn build_window(app: &AppHandle, label: &str, corner: Option<Position>) -> Result<(), DesktopError> {
    let monitor = app.primary_monitor().ok().flatten();
    let (width, height) = crate::first_window_size(monitor);
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title(title(None))
        .inner_size(width, height)
        .min_inner_size(640.0, 480.0);
    // The first window is centred; a tab dropped outside every window opens one where it was
    // dropped; other windows open where the OS cascades them.
    let builder = match corner {
        Some(Position::Logical(at)) => builder.position(at.x, at.y),
        // The builder takes logical pixels at a scale it guesses before the window has a screen:
        // a physical corner is set once it has one, before it shows.
        Some(Position::Physical(_)) => builder.visible(false),
        None if label == MAIN_WINDOW => builder.center(),
        None => builder,
    };
    let window = builder.build()?;
    if let Some(at @ Position::Physical(_)) = corner {
        // A window that can't be placed still shows, where the OS put it: a hidden one would
        // count as a window ("Move tab to window ▸") that nobody can see.
        if let Err(error) = window.set_position(at) {
            tracing::warn!("could not place a new window under the cursor: {error}");
        }
        if let Err(error) = window.show() {
            let _ = window.destroy();
            return Err(error.into());
        }
    }
    Ok(())
}

/// A new, empty window (File ▸ New Window, or a tab moved out), with its corner at `corner` when
/// given. Returns its label.
pub(crate) fn new_window(app: &AppHandle, corner: Option<Position>) -> Result<String, DesktopError> {
    let label = app.state::<Windows>().next_label();
    build_window(app, &label, corner)?;
    changed(app);
    Ok(label)
}

/// A window's title: the shown document's name, or the app's alone.
pub(crate) fn title(document: Option<&str>) -> String {
    match document.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => format!("{name} — Lamina PDF"),
        None => "Lamina PDF".to_owned(),
    }
}

/// Moves `doc` from `from` to `to`: the engine re-homes it (keeping a kept password), the file
/// watch follows it, and the new window gets it with `view`, at once or when its webview is
/// ready. A new window opens with its corner at `corner` when given (a tab dropped outside every
/// window). Returns the label of the window the tab went to.
pub(crate) fn move_tab(
    app: &AppHandle,
    from: &str,
    doc: DocId,
    to: &MoveTo,
    view: View,
    corner: Option<Position>,
) -> Result<String, DesktopError> {
    let target = match to {
        MoveTo::NewWindow => new_window(app, corner)?,
        MoveTo::Window(label) if label != from && app.get_webview_window(label).is_some() => label.clone(),
        MoveTo::Window(_) => return Err(DesktopError::UnknownWindow),
    };
    let engines = Arc::clone(&app.state::<Arc<Engines>>());
    let moved = engines.move_doc(doc, from, &target);
    let document = match moved {
        Ok(document) => document,
        Err(error) => {
            if *to == MoveTo::NewWindow
                && let Some(window) = app.get_webview_window(&target)
            {
                let _ = window.close();
            }
            return Err(error);
        }
    };
    crate::opened::move_watch(app, doc, from, &target);
    let arrived = Arrived { document, view };
    if let Some(arrived) = app.state::<Windows>().offer(&target, arrived) {
        send_arrived(app, &target, &arrived);
    }
    crate::open::bring_forward(app, &target);
    Ok(target)
}

fn send_arrived(app: &AppHandle, label: &str, arrived: &Arrived) {
    if let Err(error) = app.emit_to(EventTarget::webview_window(label), ARRIVED_EVENT, arrived) {
        tracing::warn!("could not hand a window its moved tab: {error}");
    }
}

/// Tells `label` to show `doc`'s tab, and brings it to the front.
pub(crate) fn activate(app: &AppHandle, label: &str, doc: DocId) {
    crate::open::bring_forward(app, label);
    if let Err(error) = app.emit_to(EventTarget::webview_window(label), ACTIVATE_EVENT, doc.0) {
        tracing::warn!("could not tell a window to show a tab: {error}");
    }
}

/// The other windows and their titles, for "Move tab to window ▸", in label order.
pub(crate) fn others(app: &AppHandle, except: &str) -> Vec<WindowEntry> {
    let mut list: Vec<WindowEntry> = app
        .webview_windows()
        .into_iter()
        .filter(|(label, _)| label != except)
        .map(|(label, window)| WindowEntry {
            title: window.title().unwrap_or_default(),
            label,
        })
        .collect();
    list.sort_by_key(|entry| window_order(&entry.label));
    list
}

/// `main` first, then `doc-<n>` by number.
fn window_order(label: &str) -> (u8, u32) {
    match label.strip_prefix("doc-").and_then(|n| n.parse().ok()) {
        Some(n) => (1, n),
        None => (0, 0),
    }
}

/// Tells every window, and the Window menu, that the windows or their titles changed.
pub(crate) fn changed(app: &AppHandle) {
    crate::menu::rebuild(app);
    if let Err(error) = app.emit(CHANGED_EVENT, ()) {
        tracing::warn!("could not tell the windows they changed: {error}");
    }
}

/// Hands a new window's webview the tabs moved to it before it was ready; later ones come at once.
pub(crate) fn take_arrivals(app: &AppHandle, label: &str) {
    for arrived in app.state::<Windows>().take(label) {
        send_arrived(app, label, &arrived);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arrived(doc: u32) -> Arrived {
        Arrived {
            document: DocumentMeta {
                doc,
                name: "a.pdf".into(),
                size: 1,
                page_count: 1,
                encrypted: false,
                needs_password: false,
                repaired: false,
                pages: Vec::new(),
            },
            view: serde_json::Value::Null,
        }
    }

    #[test]
    fn labels_count_up_and_are_never_reused() {
        let windows = Windows::default();
        assert_eq!(windows.next_label(), "doc-1");
        windows.closed("doc-1");
        assert_eq!(windows.next_label(), "doc-2");
    }

    #[test]
    fn a_closed_frontmost_window_hands_the_front_to_the_one_focused_before() {
        let windows = Windows::default();
        windows.focused("doc-1");
        windows.focused("main");
        assert_eq!(windows.frontmost().as_deref(), Some("main"));
        windows.closed("main");
        assert_eq!(windows.frontmost().as_deref(), Some("doc-1"));
        windows.closed("doc-1");
        assert_eq!(windows.frontmost(), None);
    }

    #[test]
    fn windows_go_front_to_back_by_when_they_were_focused() {
        let windows = Windows::default();
        windows.focused("main");
        windows.focused("doc-1");
        windows.focused("doc-2");
        windows.focused("doc-1");
        windows.closed("doc-2");
        assert_eq!(windows.front_to_back(), ["doc-1", "main"]);
    }

    #[test]
    fn tabs_wait_for_their_window_then_go_at_once() {
        let windows = Windows::default();
        assert!(windows.offer("doc-1", arrived(1)).is_none());
        assert!(windows.offer("doc-1", arrived(2)).is_none());
        let taken: Vec<u32> = windows.take("doc-1").iter().map(|a| a.document.doc).collect();
        assert_eq!(taken, [1, 2]);
        assert!(windows.offer("doc-1", arrived(3)).is_some());
        // Another window still waits for its own webview.
        assert!(windows.offer("doc-2", arrived(4)).is_none());
    }

    #[test]
    fn a_closed_window_drops_what_waited_for_it() {
        let windows = Windows::default();
        assert!(windows.offer("doc-1", arrived(1)).is_none());
        windows.closed("doc-1");
        assert!(windows.take("doc-1").is_empty());
    }

    #[test]
    fn titles_name_the_shown_document() {
        assert_eq!(title(Some("report.pdf")), "report.pdf — Lamina PDF");
        assert_eq!(title(Some("  ")), "Lamina PDF");
        assert_eq!(title(None), "Lamina PDF");
    }

    #[test]
    fn windows_list_main_first_then_by_number() {
        let mut labels = ["doc-10", "doc-2", "main"];
        labels.sort_by_key(|label| window_order(label));
        assert_eq!(labels, ["main", "doc-2", "doc-10"]);
    }

    #[test]
    fn a_move_names_its_target_as_the_webview_sends_it() {
        let new: MoveTo = serde_json::from_str(r#"{ "to": "newWindow" }"#).unwrap();
        let other: MoveTo = serde_json::from_str(r#"{ "to": "window", "label": "doc-3" }"#).unwrap();
        assert_eq!((new, other), (MoveTo::NewWindow, MoveTo::Window("doc-3".into())));
    }
}
