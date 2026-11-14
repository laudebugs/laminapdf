//! Bringing the last session back on launch: which windows to make, and for each the tabs its
//! webview gets ([`plan`]). A tab reaches the webview as a token, its file's name and its view,
//! never its path; the webview opens the shown tab at once and the others when they are first
//! shown (`session_open`), so a session of many large files starts fast. A file neither its
//! bookmark nor its path finds still comes back as a tab: opening it fails, and it offers Locate….

use serde::Serialize;

use super::track::Tracker;
use super::{Bookmarks, Resolved, SessionFile, resolve};
use crate::windows::View;

/// A restored tab, as its window's webview gets it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoredTab {
    pub token: String,
    pub name: String,
    pub view: View,
}

/// A window to make again, with its tabs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoredWindow {
    #[serde(skip)]
    pub label: String,
    pub tabs: Vec<RestoredTab>,
    pub active: Option<usize>,
}

/// Whether `label` is one the app makes: `main` or `doc-<n>` (the capability allows no other).
pub(crate) fn valid_label(label: &str) -> bool {
    label == crate::MAIN_WINDOW
        || label
            .strip_prefix("doc-")
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The windows `session` brings back, each tab resolved through `bookmarks` and kept in `tracker`
/// under its token. A file found elsewhere through its bookmark is restored from there. Windows
/// with a label the app wouldn't make, and a second window of one label, are left out.
pub(crate) fn plan(session: SessionFile, bookmarks: &dyn Bookmarks, tracker: &mut Tracker) -> Vec<RestoredWindow> {
    let mut next = 1_u32;
    let mut seen = std::collections::BTreeSet::new();
    let mut windows = Vec::new();
    for record in session.windows {
        if !valid_label(&record.label) || !seen.insert(record.label.clone()) {
            continue;
        }
        let mut tabs = Vec::with_capacity(record.tabs.len());
        for mut tab in record.tabs {
            // A file found elsewhere through its bookmark is restored from there.
            if let Resolved::Moved(path) = resolve(&tab, bookmarks) {
                tab.path = path;
            }
            let name = tab
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            let token = format!("t{next}");
            next += 1;
            tabs.push(RestoredTab {
                token: token.clone(),
                name,
                view: tab.view.clone(),
            });
            tracker.restored(token, &record.label, tab);
        }
        let active = record.active.filter(|&index| index < tabs.len());
        windows.push(RestoredWindow {
            label: record.label,
            tabs,
            active,
        });
    }
    windows
}

/// The highest `doc-<n>` among `windows`, so new windows' labels count on from it.
pub(crate) fn last_number(windows: &[RestoredWindow]) -> u32 {
    windows
        .iter()
        .filter_map(|window| window.label.strip_prefix("doc-")?.parse().ok())
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::session::{Stamp, TabRef, VERSION, WindowRecord};

    struct NoBookmarks;

    impl Bookmarks for NoBookmarks {
        fn resolve(&self, _: &[u8]) -> Option<PathBuf> {
            None
        }
    }

    fn file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-restore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, b"%PDF").unwrap();
        path
    }

    fn tab(path: &Path, page: u32) -> TabRef {
        TabRef {
            path: path.to_owned(),
            stamp: Stamp { len: 4, mtime: None },
            bookmark: None,
            view: serde_json::json!({ "page": page }),
        }
    }

    fn window(label: &str, tabs: Vec<TabRef>, active: Option<usize>) -> WindowRecord {
        WindowRecord {
            label: label.into(),
            tabs,
            active,
        }
    }

    #[test]
    fn each_tab_should_come_back_as_a_token_with_its_name_and_view_never_its_path() {
        let a = file("report.pdf");
        let session = SessionFile {
            v: VERSION,
            windows: vec![window("main", vec![tab(&a, 3)], Some(0))],
        };
        let mut tracker = Tracker::default();
        let planned = plan(session, &NoBookmarks, &mut tracker);
        let shown = serde_json::to_string(&planned[0]).unwrap();
        assert_eq!(
            (
                planned[0].tabs[0].name.as_str(),
                tracker.restored_tab("t1", "main").map(|t| t.path.clone()),
                shown.contains(&*a.to_string_lossy()),
            ),
            ("report.pdf", Some(a), false)
        );
    }

    #[test]
    fn a_file_that_is_gone_should_still_come_back_as_a_tab() {
        let session = SessionFile {
            v: VERSION,
            windows: vec![window("main", vec![tab(Path::new("/nowhere/gone.pdf"), 1)], Some(0))],
        };
        let planned = plan(session, &NoBookmarks, &mut Tracker::default());
        assert_eq!(planned[0].tabs[0].name.as_str(), "gone.pdf");
    }

    #[test]
    fn windows_with_labels_the_app_wouldnt_make_or_twice_should_be_left_out() {
        let session = SessionFile {
            v: VERSION,
            windows: vec![
                window("main", Vec::new(), None),
                window("evil", Vec::new(), None),
                window("doc-3", Vec::new(), None),
                window("doc-3", Vec::new(), None),
                window("doc-", Vec::new(), None),
            ],
        };
        let planned = plan(session, &NoBookmarks, &mut Tracker::default());
        let labels: Vec<&str> = planned.iter().map(|w| w.label.as_str()).collect();
        assert_eq!((labels, last_number(&planned)), (vec!["main", "doc-3"], 3));
    }

    #[test]
    fn an_active_index_past_the_tabs_should_be_dropped() {
        let session = SessionFile {
            v: VERSION,
            windows: vec![window("main", vec![tab(&file("e.pdf"), 1)], Some(5))],
        };
        assert_eq!(plan(session, &NoBookmarks, &mut Tracker::default())[0].active, None);
    }

    #[test]
    fn tokens_should_be_unique_across_windows() {
        let a = file("f.pdf");
        let session = SessionFile {
            v: VERSION,
            windows: vec![
                window("main", vec![tab(&a, 1), tab(&a, 2)], None),
                window("doc-1", vec![tab(&a, 3)], None),
            ],
        };
        let planned = plan(session, &NoBookmarks, &mut Tracker::default());
        let tokens: Vec<&str> = planned.iter().flat_map(|w| &w.tabs).map(|t| t.token.as_str()).collect();
        assert_eq!(tokens, ["t1", "t2", "t3"]);
    }
}
