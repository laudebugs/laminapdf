//! What the session is now: each window's webview reports its tabs ([`Tracker::report`]), and the
//! main process turns a copy of the reports into a [`SessionFile`] when it saves ([`snapshot`]),
//! every [`SAVE_EVERY`] while something changed and once more when the app quits ([`Session`]).
//!
//! A window closed by the user leaves the session; one that closes because the app quits stays,
//! so the session the user quit with comes back. On Windows and Linux closing the last window
//! quits the app, so it stays too. Its documents close before the app saves, so each reported
//! document's file is remembered when it is reported.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use laminapdf_desktop_protocol::DocId;
use serde::Deserialize;

use super::{SessionFile, Stamp, TabRef, VERSION, WindowRecord};
use crate::windows::View;

/// How often a changed session is written while the app runs.
#[cfg(not(feature = "e2e"))]
pub(crate) const SAVE_EVERY: Duration = Duration::from_secs(30);
/// End-to-end builds save every 2 s: a scenario's run can end with the app killed, which leaves
/// no quit to save on, so it waits for the saver instead.
#[cfg(feature = "e2e")]
pub(crate) const SAVE_EVERY: Duration = Duration::from_secs(2);

/// One tab as a window's webview reports it: its open document, or the token of a restored tab
/// that hasn't opened yet, and its view.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportedTab {
    #[serde(default)]
    pub doc: Option<u32>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub view: View,
}

/// One window's report: its tabs and the shown one's index.
#[derive(Debug, Clone, Default)]
pub(crate) struct Reported {
    tabs: Vec<ReportedTab>,
    active: Option<usize>,
}

/// A restored tab that hasn't opened: the window it belongs to and its reference.
#[derive(Debug, Clone)]
struct Restored {
    window: String,
    tab: TabRef,
}

/// A copy of the reports, taken under the lock, from which a save builds the file.
#[derive(Debug, Clone, Default)]
pub(crate) struct Reports {
    windows: BTreeMap<String, Reported>,
    restored: BTreeMap<String, Restored>,
    known: BTreeMap<u32, PathBuf>,
}

/// The session as the windows reported it.
#[derive(Debug, Default)]
pub(crate) struct Tracker {
    reports: Reports,
    quitting: bool,
    dirty: bool,
}

impl Tracker {
    /// `label`'s tabs now, and the shown one's index.
    pub(crate) fn report(&mut self, label: &str, tabs: Vec<ReportedTab>, active: Option<usize>) {
        self.reports.windows.insert(label.to_owned(), Reported { tabs, active });
        self.dirty = true;
    }

    /// `doc` is the file at `path`: kept so a save after `doc` closed (the app quitting) still knows it.
    pub(crate) fn remember(&mut self, doc: DocId, path: PathBuf) {
        self.reports.known.insert(doc.0, path);
    }

    /// `label` closed. It leaves the session unless the app is quitting or it was the last window
    /// on an OS where closing the last window quits.
    pub(crate) fn closed(&mut self, label: &str, last_quits: bool) {
        if self.quitting || (last_quits && self.reports.windows.len() == 1) {
            return;
        }
        if self.reports.windows.remove(label).is_some() {
            self.dirty = true;
        }
    }

    /// The app is quitting: windows that close from now on stay in the session.
    pub(crate) fn quitting(&mut self) {
        self.quitting = true;
    }

    /// Marks the session changed, so the next save writes it.
    pub(crate) fn report_changed(&mut self) {
        self.dirty = true;
    }

    /// Keeps window `window`'s restored tab `token` until it opens.
    pub(crate) fn restored(&mut self, token: String, window: &str, tab: TabRef) {
        self.reports.restored.insert(
            token,
            Restored {
                window: window.to_owned(),
                tab,
            },
        );
    }

    /// Restored tab `token`, if it is window `window`'s and hasn't opened.
    pub(crate) fn restored_tab(&self, token: &str, window: &str) -> Option<&TabRef> {
        self.reports
            .restored
            .get(token)
            .filter(|kept| kept.window == window)
            .map(|kept| &kept.tab)
    }

    /// Restored tab `token` opened: its document stands for it from now on.
    pub(crate) fn opened(&mut self, token: &str) {
        self.reports.restored.remove(token);
    }

    /// Whether something changed since the last call; clears the mark.
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// A copy of the reports, for a save to build its file outside the lock.
    pub(crate) fn reports(&self) -> Reports {
        self.reports.clone()
    }
}

/// The session file for `reports`, in `order`'s window order. `path_of` names an open document's
/// file (else the one remembered for it), and `bookmark` makes a file's bookmark (none outside
/// macOS); a restored tab gets a fresh one from where its file is now, else keeps its own. A tab
/// whose file is unknown or gone is left out.
pub(crate) fn snapshot<K: Ord>(
    reports: &Reports,
    order: impl Fn(&str) -> K,
    path_of: impl Fn(DocId) -> Option<PathBuf>,
    mut bookmark: impl FnMut(&Path) -> Option<String>,
) -> SessionFile {
    let mut labels: Vec<&String> = reports.windows.keys().collect();
    labels.sort_by_key(|label| order(label));
    let mut windows = Vec::with_capacity(labels.len());
    for label in labels {
        let Some(reported) = reports.windows.get(label) else {
            continue;
        };
        let mut tabs = Vec::new();
        let mut active = None;
        for (index, tab) in reported.tabs.iter().enumerate() {
            let reference = match (tab.doc, tab.token.as_deref()) {
                (Some(doc), _) => path_of(DocId(doc))
                    .or_else(|| reports.known.get(&doc).cloned())
                    .and_then(|path| {
                        let path = std::fs::canonicalize(&path).unwrap_or(path);
                        let stamp = Stamp::of(&path).ok()?;
                        let bookmark = bookmark(&path);
                        Some(TabRef {
                            path,
                            stamp,
                            bookmark,
                            view: tab.view.clone(),
                        })
                    }),
                (None, Some(token)) => reports.restored.get(token).map(|kept| TabRef {
                    bookmark: bookmark(&kept.tab.path).or_else(|| kept.tab.bookmark.clone()),
                    view: tab.view.clone(),
                    ..kept.tab.clone()
                }),
                (None, None) => None,
            };
            if let Some(reference) = reference {
                if reported.active == Some(index) {
                    active = Some(tabs.len());
                }
                tabs.push(reference);
            }
        }
        windows.push(WindowRecord {
            label: label.clone(),
            tabs,
            active,
        });
    }
    SessionFile { v: VERSION, windows }
}

/// The session, where it is saved, and the bookmarks made already (making one reads the disk).
pub(crate) struct Session {
    dir: PathBuf,
    tracker: Mutex<Tracker>,
    bookmarks: Mutex<BTreeMap<PathBuf, Option<String>>>,
}

impl Session {
    pub(crate) fn new(dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            dir,
            tracker: Mutex::new(Tracker::default()),
            bookmarks: Mutex::new(BTreeMap::new()),
        })
    }

    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn tracker(&self) -> MutexGuard<'_, Tracker> {
        // Maps of reports: nothing a panic could leave half-done.
        self.tracker.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The bookmarks made already, by path; only a save takes them, never under the tracker's lock.
    pub(crate) fn bookmarks(&self) -> MutexGuard<'_, BTreeMap<PathBuf, Option<String>>> {
        self.bookmarks.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-track-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, b"%PDF").unwrap();
        std::fs::canonicalize(path).unwrap()
    }

    fn doc_tab(doc: u32, page: u32) -> ReportedTab {
        ReportedTab {
            doc: Some(doc),
            token: None,
            view: serde_json::json!({ "page": page }),
        }
    }

    fn order(label: &str) -> String {
        label.to_owned()
    }

    fn snap(tracker: &Tracker, path_of: impl Fn(DocId) -> Option<PathBuf>) -> SessionFile {
        snapshot(&tracker.reports(), order, path_of, |_| None)
    }

    fn kept(path: &Path) -> TabRef {
        TabRef {
            path: path.to_owned(),
            stamp: Stamp { len: 4, mtime: None },
            bookmark: Some("ab".into()),
            view: serde_json::json!({ "page": 1 }),
        }
    }

    #[test]
    fn a_snapshot_should_hold_each_reported_tab_with_its_file_and_view() {
        let a = temp_file("a.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(1, 4)], Some(0));
        let snap = snap(&tracker, |doc| (doc == DocId(1)).then(|| a.clone()));
        let tab = &snap.windows[0].tabs[0];
        assert_eq!(
            (snap.windows[0].active, &tab.path, &tab.view, tab.stamp.len),
            (Some(0), &a, &serde_json::json!({ "page": 4 }), 4)
        );
    }

    #[test]
    fn a_document_closed_before_the_save_should_keep_its_remembered_file() {
        let a = temp_file("closed.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(1, 2)], Some(0));
        tracker.remember(DocId(1), a.clone());
        // The last window closed, its documents with it, and the app quit.
        tracker.closed("main", true);
        let snap = snap(&tracker, |_| None);
        assert_eq!(snap.windows[0].tabs.first().map(|t| &t.path), Some(&a));
    }

    #[test]
    fn a_tab_whose_file_is_unknown_should_be_left_out_and_the_active_index_follow() {
        let a = temp_file("b.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(9, 1), doc_tab(1, 2)], Some(1));
        let snap = snap(&tracker, |doc| (doc == DocId(1)).then(|| a.clone()));
        assert_eq!((snap.windows[0].tabs.len(), snap.windows[0].active), (1, Some(0)));
    }

    #[test]
    fn a_restored_tab_not_yet_opened_should_keep_its_reference_with_its_new_view() {
        let a = temp_file("c.pdf");
        let mut tracker = Tracker::default();
        tracker.restored("t1".into(), "main", kept(&a));
        let reported = ReportedTab {
            doc: None,
            token: Some("t1".into()),
            view: serde_json::json!({ "page": 7 }),
        };
        tracker.report("main", vec![reported], Some(0));
        let snap = snap(&tracker, |_| None);
        let tab = &snap.windows[0].tabs[0];
        assert_eq!(
            (&tab.path, tab.bookmark.as_deref(), &tab.view),
            (&a, Some("ab"), &serde_json::json!({ "page": 7 }))
        );
    }

    #[test]
    fn a_restored_tab_should_get_a_fresh_bookmark_when_one_can_be_made() {
        let a = temp_file("fresh.pdf");
        let mut tracker = Tracker::default();
        tracker.restored("t1".into(), "main", kept(&a));
        let reported = ReportedTab {
            doc: None,
            token: Some("t1".into()),
            view: View::Null,
        };
        tracker.report("main", vec![reported], None);
        let snap = snapshot(&tracker.reports(), order, |_| None, |_| Some("new".into()));
        assert_eq!(snap.windows[0].tabs[0].bookmark.as_deref(), Some("new"));
    }

    #[test]
    fn a_restored_tab_should_be_another_windows_to_open_and_gone_once_opened() {
        let mut tracker = Tracker::default();
        tracker.restored("t1".into(), "main", kept(Path::new("/x.pdf")));
        let elsewhere = tracker.restored_tab("t1", "doc-1").is_some();
        let here = tracker.restored_tab("t1", "main").is_some();
        tracker.opened("t1");
        assert_eq!(
            (elsewhere, here, tracker.restored_tab("t1", "main").is_some()),
            (false, true, false)
        );
    }

    #[test]
    fn windows_should_come_in_the_given_order() {
        let mut tracker = Tracker::default();
        for label in ["doc-2", "main", "doc-1"] {
            tracker.report(label, Vec::new(), None);
        }
        let rank = |label: &str| match label {
            "main" => 0,
            "doc-1" => 1,
            _ => 2,
        };
        let snap = snapshot(&tracker.reports(), rank, |_| None, |_| None);
        let labels: Vec<&str> = snap.windows.iter().map(|w| w.label.as_str()).collect();
        assert_eq!(labels, ["main", "doc-1", "doc-2"]);
    }

    #[test]
    fn a_window_the_user_closes_should_leave_the_session() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.report("doc-1", Vec::new(), None);
        tracker.closed("doc-1", false);
        assert_eq!(snap(&tracker, |_| None).windows.len(), 1);
    }

    #[test]
    fn windows_closing_while_the_app_quits_should_stay() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.report("doc-1", Vec::new(), None);
        tracker.quitting();
        tracker.closed("doc-1", false);
        tracker.closed("main", false);
        assert_eq!(snap(&tracker, |_| None).windows.len(), 2);
    }

    #[test]
    fn the_last_window_should_stay_where_closing_it_quits_the_app() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.closed("main", true);
        let quits = snap(&tracker, |_| None).windows.len();
        tracker.closed("main", false);
        let stays_open = snap(&tracker, |_| None).windows.len();
        assert_eq!((quits, stays_open), (1, 0));
    }

    #[test]
    fn a_report_should_mark_the_session_changed_until_it_is_taken() {
        let mut tracker = Tracker::default();
        let before = tracker.take_dirty();
        tracker.report("main", Vec::new(), None);
        assert_eq!(
            [before, tracker.take_dirty(), tracker.take_dirty()],
            [false, true, false]
        );
    }
}
