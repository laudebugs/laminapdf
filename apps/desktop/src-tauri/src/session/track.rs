//! What the session is now: each window's webview reports its tabs ([`Tracker::report`]), and the
//! main process turns them into a [`SessionFile`] when it saves, every 30 s while something
//! changed and once more when the app quits ([`Session`]).
//!
//! A window closed by the user leaves the session; one that closes because the app quits stays,
//! so the session the user quit with comes back. On Windows and Linux closing the last window
//! quits the app, so it stays too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use laminapdf_desktop_protocol::DocId;
use serde::Deserialize;

use super::{SessionFile, Stamp, TabRef, VERSION, WindowRecord};
use crate::windows::View;

/// How often a changed session is written while the app runs.
pub(crate) const SAVE_EVERY: Duration = Duration::from_secs(30);

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

#[derive(Debug, Clone, Default)]
struct Reported {
    tabs: Vec<ReportedTab>,
    active: Option<usize>,
}

/// The session as the windows reported it.
#[derive(Debug, Default)]
pub(crate) struct Tracker {
    windows: BTreeMap<String, Reported>,
    /// Restored tabs by token, as the session file had them, until each is opened.
    restored: BTreeMap<String, TabRef>,
    /// Bookmarks made already, by path: making one reads the file system.
    bookmarks: BTreeMap<PathBuf, Option<String>>,
    quitting: bool,
    dirty: bool,
}

impl Tracker {
    /// `label`'s tabs now, and the shown one's index.
    pub(crate) fn report(&mut self, label: &str, tabs: Vec<ReportedTab>, active: Option<usize>) {
        self.windows.insert(label.to_owned(), Reported { tabs, active });
        self.dirty = true;
    }

    /// `label` closed. It leaves the session unless the app is quitting or it was the last window
    /// on an OS where closing the last window quits.
    pub(crate) fn closed(&mut self, label: &str, last_quits: bool) {
        if self.quitting || (last_quits && self.windows.len() == 1) {
            return;
        }
        if self.windows.remove(label).is_some() {
            self.dirty = true;
        }
    }

    /// Marks the session changed, so the next save writes it.
    pub(crate) fn report_changed(&mut self) {
        self.dirty = true;
    }

    /// The app is quitting: windows that close from now on stay in the session.
    pub(crate) fn quitting(&mut self) {
        self.quitting = true;
    }

    /// Keeps a restored tab's reference until its tab opens it.
    pub(crate) fn restored(&mut self, token: String, tab: TabRef) {
        self.restored.insert(token, tab);
    }

    /// The restored tab `token` stands for.
    pub(crate) fn restored_tab(&self, token: &str) -> Option<&TabRef> {
        self.restored.get(token)
    }

    /// Whether something changed since the last [`Tracker::snapshot`]; clears the mark.
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// The session file for what the windows reported, in `order`'s window order. `path_of` names
    /// an open document's file and `bookmark` makes a file's bookmark (none outside macOS). A tab
    /// whose file is unknown (its document closed meanwhile) is left out.
    pub(crate) fn snapshot<K: Ord>(
        &mut self,
        order: impl Fn(&str) -> K,
        path_of: impl Fn(DocId) -> Option<PathBuf>,
        bookmark: impl Fn(&Path) -> Option<String>,
    ) -> SessionFile {
        let mut labels: Vec<&String> = self.windows.keys().collect();
        labels.sort_by_key(|label| order(label));
        let mut windows = Vec::with_capacity(labels.len());
        for label in labels {
            let Some(reported) = self.windows.get(label) else {
                continue;
            };
            let mut tabs = Vec::new();
            let mut active = None;
            for (index, tab) in reported.tabs.iter().enumerate() {
                let reference = match (tab.doc, tab.token.as_deref()) {
                    (Some(doc), _) => path_of(DocId(doc)).and_then(|path| {
                        let path = std::fs::canonicalize(&path).unwrap_or(path);
                        let bookmark = self
                            .bookmarks
                            .entry(path.clone())
                            .or_insert_with(|| bookmark(&path))
                            .clone();
                        let stamp = Stamp::of(&path).ok()?;
                        Some(TabRef {
                            path,
                            stamp,
                            bookmark,
                            view: tab.view.clone(),
                        })
                    }),
                    (None, Some(token)) => self.restored.get(token).map(|kept| TabRef {
                        view: tab.view.clone(),
                        ..kept.clone()
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
}

/// The session and where it is saved.
pub(crate) struct Session {
    dir: PathBuf,
    tracker: Mutex<Tracker>,
}

impl Session {
    pub(crate) fn new(dir: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            dir,
            tracker: Mutex::new(Tracker::default()),
        })
    }

    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn tracker(&self) -> MutexGuard<'_, Tracker> {
        // Maps of reports: nothing a panic could leave half-done.
        self.tracker.lock().unwrap_or_else(PoisonError::into_inner)
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

    #[test]
    fn a_snapshot_should_hold_each_reported_tab_with_its_file_and_view() {
        let a = temp_file("a.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(1, 4)], Some(0));
        let snap = tracker.snapshot(order, |doc| (doc == DocId(1)).then(|| a.clone()), |_| None);
        let tab = &snap.windows[0].tabs[0];
        assert_eq!(
            (snap.windows[0].active, &tab.path, &tab.view, tab.stamp.len),
            (Some(0), &a, &serde_json::json!({ "page": 4 }), 4)
        );
    }

    #[test]
    fn a_tab_whose_file_is_unknown_should_be_left_out_and_the_active_index_follow() {
        let a = temp_file("b.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(9, 1), doc_tab(1, 2)], Some(1));
        let snap = tracker.snapshot(order, |doc| (doc == DocId(1)).then(|| a.clone()), |_| None);
        assert_eq!((snap.windows[0].tabs.len(), snap.windows[0].active), (1, Some(0)));
    }

    #[test]
    fn a_restored_tab_not_yet_opened_should_keep_its_reference_with_its_new_view() {
        let a = temp_file("c.pdf");
        let mut tracker = Tracker::default();
        let kept = TabRef {
            path: a.clone(),
            stamp: Stamp { len: 4, mtime: None },
            bookmark: Some("ab".into()),
            view: serde_json::json!({ "page": 1 }),
        };
        tracker.restored("t1".into(), kept);
        let reported = ReportedTab {
            doc: None,
            token: Some("t1".into()),
            view: serde_json::json!({ "page": 7 }),
        };
        tracker.report("main", vec![reported], Some(0));
        let snap = tracker.snapshot(order, |_| None, |_| None);
        let tab = &snap.windows[0].tabs[0];
        assert_eq!(
            (&tab.path, tab.bookmark.as_deref(), &tab.view),
            (&a, Some("ab"), &serde_json::json!({ "page": 7 }))
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
        let snap = tracker.snapshot(rank, |_| None, |_| None);
        let labels: Vec<&str> = snap.windows.iter().map(|w| w.label.as_str()).collect();
        assert_eq!(labels, ["main", "doc-1", "doc-2"]);
    }

    #[test]
    fn a_window_the_user_closes_should_leave_the_session() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.report("doc-1", Vec::new(), None);
        tracker.closed("doc-1", false);
        let snap = tracker.snapshot(order, |_| None, |_| None);
        assert_eq!(snap.windows.len(), 1);
    }

    #[test]
    fn windows_closing_while_the_app_quits_should_stay() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.report("doc-1", Vec::new(), None);
        tracker.quitting();
        tracker.closed("doc-1", false);
        tracker.closed("main", false);
        assert_eq!(tracker.snapshot(order, |_| None, |_| None).windows.len(), 2);
    }

    #[test]
    fn the_last_window_should_stay_where_closing_it_quits_the_app() {
        let mut tracker = Tracker::default();
        tracker.report("main", Vec::new(), None);
        tracker.closed("main", true);
        let quits = tracker.snapshot(order, |_| None, |_| None).windows.len();
        tracker.closed("main", false);
        let stays_open = tracker.snapshot(order, |_| None, |_| None).windows.len();
        assert_eq!((quits, stays_open), (1, 0));
    }

    #[test]
    fn a_bookmark_should_be_made_once_per_file() {
        let a = temp_file("d.pdf");
        let mut tracker = Tracker::default();
        tracker.report("main", vec![doc_tab(1, 1)], Some(0));
        let made = std::cell::Cell::new(0);
        for _ in 0..3 {
            tracker.snapshot(
                order,
                |_| Some(a.clone()),
                |_| {
                    made.set(made.get() + 1);
                    Some("bm".into())
                },
            );
        }
        assert_eq!(made.get(), 1);
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
