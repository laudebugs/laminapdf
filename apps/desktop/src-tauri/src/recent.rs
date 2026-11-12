//! Recent files: the in-app list on the home screen, kept in `recent.json` in the app's config
//! folder, and the OS's own lists (the Dock and File ▸ Open Recent, the Jump List, GNOME Files'
//! Recent) through `laminapdf-desktop-os`.
//!
//! Paths stay in the main process: the webview sees each entry's id, file name, size and dates,
//! and opens one by its id (`recent_open`). The list is local only (ADR 0011): nothing in it is
//! sent anywhere. Turning "Remember recent files" off clears both lists and stops recording.

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::store::{read_json, write_json};

/// How many files the list keeps; the oldest go.
pub(crate) const MAX_ENTRIES: usize = 20;

/// The file's format version.
const VERSION: u32 = 1;

/// One recent file, as `recent.json` keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    /// Never sent to the webview.
    path: PathBuf,
    name: String,
    size: u64,
    /// Milliseconds since the epoch.
    last_opened: u64,
    pages: Option<u32>,
    /// The key its reading position is kept under (`Platform.positions`).
    fingerprint: Option<String>,
}

/// What `recent.json` holds.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Saved {
    version: u32,
    entries: Vec<Entry>,
}

/// One entry as the webview sees it: `RecentFile` in `@laminapdf/ui`, without its path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Listed {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub last_opened: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// Opening it needs no dialog: the file is where it was.
    pub has_handle: bool,
    /// The file is no longer where it was.
    pub missing: bool,
}

/// A file that just opened, for [`RecentFiles::add`].
pub(crate) struct Opened<'a> {
    pub path: &'a Path,
    pub name: &'a str,
    pub size: u64,
    pub pages: Option<u32>,
    pub fingerprint: Option<String>,
}

/// The in-app recent list, saved after every change.
pub(crate) struct RecentFiles {
    /// `None` in tests that keep it in memory.
    file: Option<PathBuf>,
    entries: Mutex<Vec<Entry>>,
}

impl RecentFiles {
    /// The list saved at `file`; empty when it is missing, and with a warning when it is damaged.
    pub(crate) fn load(file: PathBuf) -> Self {
        let saved: Option<Saved> = read_json(&file, "recent files list");
        let entries = saved
            .filter(|saved| saved.version == VERSION)
            .map(|saved| saved.entries)
            .unwrap_or_default();
        Self {
            file: Some(file),
            entries: Mutex::new(entries),
        }
    }

    /// A list kept in memory only.
    #[cfg(test)]
    fn in_memory() -> Self {
        Self {
            file: None,
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Puts `opened` first, replacing an older entry for the same file (by its canonical path,
    /// which keeps the entry's id), and drops the oldest past [`MAX_ENTRIES`]. Returns the id.
    pub(crate) fn add(&self, opened: Opened<'_>, now: u64) -> String {
        let path = canonical(opened.path);
        let mut entries = self.lock();
        let id = match entries.iter().position(|entry| entry.path == path) {
            Some(at) => entries.remove(at).id,
            None => id_of(&path),
        };
        entries.insert(
            0,
            Entry {
                id: id.clone(),
                path,
                name: opened.name.to_owned(),
                size: opened.size,
                last_opened: now,
                pages: opened.pages,
                fingerprint: opened.fingerprint,
            },
        );
        entries.truncate(MAX_ENTRIES);
        self.save(&entries);
        id
    }

    /// Newest first, each marked missing when its file is gone. The disk is checked after the list
    /// is copied, so a slow drive never holds up `add` or `recent_open`.
    pub(crate) fn list(&self) -> Vec<Listed> {
        let entries = self.lock().clone();
        entries
            .iter()
            .map(|entry| {
                let missing = !entry.path.is_file();
                Listed {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    size: entry.size,
                    last_opened: entry.last_opened,
                    pages: entry.pages,
                    fingerprint: entry.fingerprint.clone(),
                    has_handle: !missing,
                    missing,
                }
            })
            .collect()
    }

    /// The path of the entry `id`, if there is one.
    pub(crate) fn path_of(&self, id: &str) -> Option<PathBuf> {
        self.lock()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.path.clone())
    }

    /// Forgets the entry `id`; returns its path, for the OS's list.
    pub(crate) fn remove(&self, id: &str) -> Option<PathBuf> {
        let mut entries = self.lock();
        let at = entries.iter().position(|entry| entry.id == id)?;
        let removed = entries.remove(at);
        self.save(&entries);
        Some(removed.path)
    }

    /// Forgets every entry; returns their paths, for the OS's list.
    pub(crate) fn clear(&self) -> Vec<PathBuf> {
        let mut entries = self.lock();
        let paths = entries.drain(..).map(|entry| entry.path).collect();
        self.save(&entries);
        paths
    }

    fn save(&self, entries: &[Entry]) {
        let Some(file) = &self.file else {
            return;
        };
        let saved = Saved {
            version: VERSION,
            entries: entries.to_vec(),
        };
        if let Err(error) = write_json(file, &saved) {
            tracing::warn!("could not save the recent files list: {error}");
        }
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Entry>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `path` with its links and `..` resolved, so two names for one file share an entry; as given
/// when it can't be resolved (gone already).
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// An entry's id: a hash of its canonical path, so the webview can name an entry without seeing
/// where it is.
fn id_of(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("r{:016x}", hasher.finish())
}

/// Milliseconds since the epoch.
pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in a fresh temporary folder, removed with it.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("laminapdf-recent-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, b"%PDF-1.7").unwrap();
            path
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn opened(path: &Path) -> Opened<'_> {
        Opened {
            path,
            name: "a.pdf",
            size: 8,
            pages: Some(1),
            fingerprint: None,
        }
    }

    fn names(recent: &RecentFiles) -> Vec<PathBuf> {
        recent.lock().iter().map(|entry| entry.path.clone()).collect()
    }

    #[test]
    fn the_newest_file_should_come_first() {
        let temp = Temp::new("order");
        let (a, b) = (temp.file("a.pdf"), temp.file("b.pdf"));
        let recent = RecentFiles::in_memory();
        recent.add(opened(&a), 1);
        recent.add(opened(&b), 2);
        assert_eq!(names(&recent), vec![canonical(&b), canonical(&a)]);
    }

    #[test]
    fn opening_a_file_again_should_move_it_first_and_keep_its_id() {
        let temp = Temp::new("dedupe");
        let (a, b) = (temp.file("a.pdf"), temp.file("b.pdf"));
        let recent = RecentFiles::in_memory();
        let first = recent.add(opened(&a), 1);
        recent.add(opened(&b), 2);
        let again = recent.add(opened(&a), 3);
        assert_eq!((names(&recent), again), (vec![canonical(&a), canonical(&b)], first));
    }

    #[test]
    fn two_names_for_one_file_should_share_an_entry() {
        let temp = Temp::new("canonical");
        let a = temp.file("a.pdf");
        let roundabout = temp.0.join("sub").join("..").join("a.pdf");
        std::fs::create_dir_all(temp.0.join("sub")).unwrap();
        let recent = RecentFiles::in_memory();
        recent.add(opened(&a), 1);
        recent.add(opened(&roundabout), 2);
        assert_eq!(recent.list().len(), 1);
    }

    #[test]
    fn the_list_should_keep_twenty_files() {
        let temp = Temp::new("cap");
        let recent = RecentFiles::in_memory();
        let files: Vec<PathBuf> = (0..25).map(|i| temp.file(&format!("{i}.pdf"))).collect();
        for (i, file) in files.iter().enumerate() {
            recent.add(opened(file), i as u64);
        }
        let kept = names(&recent);
        assert_eq!((kept.len(), kept.first()), (MAX_ENTRIES, Some(&canonical(&files[24]))));
    }

    #[test]
    fn a_saved_list_should_load_again() {
        let temp = Temp::new("persist");
        let a = temp.file("a.pdf");
        let file = temp.0.join("config").join("recent.json");
        let id = RecentFiles::load(file.clone()).add(opened(&a), 7);
        let listed = RecentFiles::load(file).list();
        assert_eq!(
            listed
                .iter()
                .map(|e| (e.id.as_str(), e.last_opened))
                .collect::<Vec<_>>(),
            vec![(id.as_str(), 7)]
        );
    }

    #[test]
    fn a_damaged_list_should_load_empty() {
        let temp = Temp::new("damaged");
        let file = temp.0.join("recent.json");
        std::fs::write(&file, b"[[[").unwrap();
        assert_eq!(RecentFiles::load(file).list(), Vec::new());
    }

    #[test]
    fn a_deleted_file_should_be_listed_as_missing() {
        let temp = Temp::new("missing");
        let a = temp.file("a.pdf");
        let recent = RecentFiles::in_memory();
        recent.add(opened(&a), 1);
        std::fs::remove_file(&a).unwrap();
        let listed = recent.list();
        assert!(listed.iter().all(|entry| entry.missing && !entry.has_handle));
    }

    #[test]
    fn the_webview_should_get_no_path() {
        let temp = Temp::new("nopath");
        let a = temp.file("a.pdf");
        let recent = RecentFiles::in_memory();
        recent.add(opened(&a), 1);
        let json = serde_json::to_string(&recent.list()).unwrap();
        assert!(
            !json.contains(&temp.0.file_name().unwrap().to_string_lossy().into_owned()),
            "{json}"
        );
    }

    #[test]
    fn removing_an_entry_should_return_its_path() {
        let temp = Temp::new("remove");
        let a = temp.file("a.pdf");
        let recent = RecentFiles::in_memory();
        let id = recent.add(opened(&a), 1);
        assert_eq!((recent.remove(&id), recent.list().len()), (Some(canonical(&a)), 0));
    }

    #[test]
    fn clearing_should_return_every_path() {
        let temp = Temp::new("clear");
        let (a, b) = (temp.file("a.pdf"), temp.file("b.pdf"));
        let recent = RecentFiles::in_memory();
        recent.add(opened(&a), 1);
        recent.add(opened(&b), 2);
        assert_eq!((recent.clear().len(), recent.list().len()), (2, 0));
    }
}
