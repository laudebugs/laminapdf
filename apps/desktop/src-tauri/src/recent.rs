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
    /// Its security-scoped bookmark, as hex: in the Mac App Store build, the only way back to the
    /// file after a relaunch (ADR 0014 §11); none in the direct download.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bookmark: Option<String>,
}

/// How an entry's file can be reached now.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Reach {
    /// At this path, with no dialog.
    Ready(PathBuf),
    /// The file is there, but App Sandbox lets the app reach it again only through a bookmark the
    /// entry lacks (macOS refused one, the list came from before bookmarks, or the bookmark no
    /// longer resolves): the user chooses it in the file picker.
    NeedsPicker,
    /// The file is gone.
    Missing,
}

/// How a file is reached: `resolved` is where its bookmark leads, if it has one that resolves;
/// `path` is where the entry last saw it. In `sandbox` (the Mac App Store build) a path alone
/// reaches nothing after a relaunch, so a file only its path names needs the picker.
fn reach(sandbox: bool, resolved: Option<PathBuf>, path: &Path) -> Reach {
    match resolved {
        Some(found) if found.is_file() => Reach::Ready(found),
        Some(_) => Reach::Missing,
        None if !path.is_file() => Reach::Missing,
        None if sandbox => Reach::NeedsPicker,
        None => Reach::Ready(path.to_owned()),
    }
}

impl Entry {
    /// Where its bookmark leads, when it has one that resolves. Resolving also keeps the file
    /// reachable in App Sandbox until the app quits, even after the entry is forgotten, since a
    /// document open from it may still need it to save.
    fn resolve(&self) -> Option<PathBuf> {
        self.bookmark.as_deref().and_then(crate::scoped::find_hex)
    }

    /// Where the file is: through its bookmark when it has one, else its path.
    fn locate(&self) -> PathBuf {
        self.resolve().unwrap_or_else(|| self.path.clone())
    }

    fn reach(&self, sandbox: bool) -> Reach {
        reach(sandbox, self.resolve(), &self.path)
    }
}

/// What opening a recent entry takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Way {
    /// Open the file at this path.
    Open(PathBuf),
    /// Ask for the file in the picker, starting in this folder when it is known.
    Choose(Option<PathBuf>),
}

/// Whether this build runs in App Sandbox.
const SANDBOX: bool = cfg!(feature = "app-store");

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
    /// Opening it needs no dialog: the file is where it was. False for a file the picker must
    /// find again (App Sandbox, no usable bookmark) and for a missing one.
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
    /// Held while a change is saved, so saves land in the order of the changes while `list` and
    /// `path_of` only wait for the change itself, never for the disk.
    saving: Mutex<()>,
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
            saving: Mutex::new(()),
        }
    }

    /// A list kept in memory only.
    #[cfg(test)]
    fn in_memory() -> Self {
        Self {
            file: None,
            entries: Mutex::new(Vec::new()),
            saving: Mutex::new(()),
        }
    }

    /// Puts `opened` first, replacing an older entry for the same file (by its canonical path, or
    /// where its bookmark now leads, for a file that moved; the entry keeps its id), and drops the
    /// oldest past [`MAX_ENTRIES`]. Returns the id. The new entry's bookmark is fresh, so a stale
    /// one is replaced each time its file opens.
    pub(crate) fn add(&self, opened: Opened<'_>, now: u64) -> String {
        let path = canonical(opened.path);
        let bookmark = crate::scoped::bookmark_hex(&path);
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = self.lock();
        let same = |entry: &Entry| entry.path == path || (entry.bookmark.is_some() && entry.locate() == path);
        let id = match entries.iter().position(same) {
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
                bookmark,
            },
        );
        entries.truncate(MAX_ENTRIES);
        let snapshot = entries.clone();
        drop(entries);
        self.save(snapshot);
        id
    }

    /// Newest first, each marked missing when its file is gone. The disk is checked after the list
    /// is copied, so a slow drive never holds up `add` or `recent_open`.
    pub(crate) fn list(&self) -> Vec<Listed> {
        self.list_as(SANDBOX)
    }

    fn list_as(&self, sandbox: bool) -> Vec<Listed> {
        let entries = self.lock().clone();
        entries
            .iter()
            .map(|entry| {
                let reach = entry.reach(sandbox);
                let missing = reach == Reach::Missing;
                Listed {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    size: entry.size,
                    last_opened: entry.last_opened,
                    pages: entry.pages,
                    fingerprint: entry.fingerprint.clone(),
                    has_handle: matches!(reach, Reach::Ready(_)),
                    missing,
                }
            })
            .collect()
    }

    /// How to open the entry `id`, if there is one. A missing file still gives its path, so
    /// opening it fails the way it always did.
    pub(crate) fn way_to_open(&self, id: &str) -> Option<Way> {
        self.way_to_open_as(id, SANDBOX)
    }

    fn way_to_open_as(&self, id: &str, sandbox: bool) -> Option<Way> {
        // Copied out, so resolving its bookmark doesn't hold the list's lock.
        let entry = self.lock().iter().find(|entry| entry.id == id).cloned()?;
        Some(match entry.reach(sandbox) {
            Reach::Ready(path) => Way::Open(path),
            Reach::NeedsPicker => Way::Choose(entry.path.parent().map(Path::to_owned)),
            Reach::Missing => Way::Open(entry.locate()),
        })
    }

    /// Forgets the entry `id`; returns its path, for the OS's list.
    pub(crate) fn remove(&self, id: &str) -> Option<PathBuf> {
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = self.lock();
        let at = entries.iter().position(|entry| entry.id == id)?;
        let removed = entries.remove(at);
        let snapshot = entries.clone();
        drop(entries);
        self.save(snapshot);
        Some(removed.path)
    }

    /// Forgets every entry; returns their paths, for the OS's list.
    pub(crate) fn clear(&self) -> Vec<PathBuf> {
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        let paths = self.lock().drain(..).map(|entry| entry.path).collect();
        self.save(Vec::new());
        paths
    }

    /// Writes `entries`; the caller holds `saving` and not the list's own lock.
    fn save(&self, entries: Vec<Entry>) {
        let Some(file) = &self.file else {
            return;
        };
        let saved = Saved {
            version: VERSION,
            entries,
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

    /// The store build keeps bookmarks, so a moved file reopened from its entry replaces it.
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    #[test]
    fn a_moved_file_should_keep_its_entry() {
        let temp = Temp::new("moved");
        let before = temp.file("before.pdf");
        let recent = RecentFiles::in_memory();
        let id = recent.add(opened(&before), 1);
        let after = temp.0.join("after.pdf");
        std::fs::rename(&before, &after).unwrap();
        let again = recent.add(opened(&after), 2);
        assert_eq!((names(&recent), again), (vec![canonical(&after)], id));
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

    /// A bookmark that no longer resolves: hex of bytes macOS never issued.
    const STALE: &str = "00ff00ff";

    fn entry_of(path: &Path, bookmark: Option<&str>) -> Entry {
        Entry {
            id: id_of(path),
            path: path.to_owned(),
            name: "a.pdf".to_owned(),
            size: 8,
            last_opened: 1,
            pages: None,
            fingerprint: None,
            bookmark: bookmark.map(str::to_owned),
        }
    }

    fn holding(entries: Vec<Entry>) -> RecentFiles {
        let recent = RecentFiles::in_memory();
        *recent.lock() = entries;
        recent
    }

    #[test]
    fn a_path_alone_should_be_ready_outside_the_sandbox() {
        let temp = Temp::new("reach-direct");
        let a = temp.file("a.pdf");
        assert_eq!(reach(false, None, &a), Reach::Ready(a));
    }

    #[test]
    fn a_path_alone_should_need_the_picker_in_the_sandbox() {
        let temp = Temp::new("reach-sandbox");
        let a = temp.file("a.pdf");
        assert_eq!(reach(true, None, &a), Reach::NeedsPicker);
    }

    #[test]
    fn a_bookmark_that_resolves_should_be_ready_in_the_sandbox() {
        let temp = Temp::new("reach-bookmark");
        let a = temp.file("a.pdf");
        assert_eq!(reach(true, Some(a.clone()), &a), Reach::Ready(a));
    }

    #[test]
    fn a_deleted_file_should_be_missing_in_the_sandbox_too() {
        let temp = Temp::new("reach-gone");
        let gone = temp.0.join("gone.pdf");
        assert_eq!(
            (reach(true, None, &gone), reach(true, Some(gone.clone()), &gone)),
            (Reach::Missing, Reach::Missing)
        );
    }

    #[test]
    fn an_entry_without_a_bookmark_should_stay_listed_and_need_the_picker_in_the_sandbox() {
        let temp = Temp::new("list-sandbox");
        let a = temp.file("a.pdf");
        let listed = holding(vec![entry_of(&a, None)]).list_as(true);
        assert_eq!(
            listed.iter().map(|e| (e.has_handle, e.missing)).collect::<Vec<_>>(),
            vec![(false, false)]
        );
    }

    #[test]
    fn an_entry_without_a_bookmark_should_open_by_its_path_outside_the_sandbox() {
        let temp = Temp::new("list-direct");
        let a = temp.file("a.pdf");
        let listed = holding(vec![entry_of(&a, None)]).list_as(false);
        assert_eq!(
            listed.iter().map(|e| (e.has_handle, e.missing)).collect::<Vec<_>>(),
            vec![(true, false)]
        );
    }

    #[test]
    fn a_bookmark_that_does_not_resolve_should_need_the_picker_in_the_sandbox() {
        let temp = Temp::new("list-stale");
        let a = temp.file("a.pdf");
        let listed = holding(vec![entry_of(&a, Some(STALE))]).list_as(true);
        assert_eq!(
            listed.iter().map(|e| (e.has_handle, e.missing)).collect::<Vec<_>>(),
            vec![(false, false)]
        );
    }

    #[test]
    fn opening_an_entry_that_needs_the_picker_should_start_in_its_folder() {
        let temp = Temp::new("way-choose");
        let a = temp.file("a.pdf");
        let entry = entry_of(&a, None);
        let id = entry.id.clone();
        assert_eq!(
            holding(vec![entry]).way_to_open_as(&id, true),
            Some(Way::Choose(Some(temp.0.clone())))
        );
    }

    #[test]
    fn opening_an_entry_outside_the_sandbox_should_use_its_path() {
        let temp = Temp::new("way-open");
        let a = temp.file("a.pdf");
        let entry = entry_of(&a, None);
        let id = entry.id.clone();
        assert_eq!(holding(vec![entry]).way_to_open_as(&id, false), Some(Way::Open(a)));
    }

    #[test]
    fn opening_a_missing_entry_should_still_try_its_path() {
        let temp = Temp::new("way-missing");
        let gone = temp.0.join("gone.pdf");
        let entry = entry_of(&gone, None);
        let id = entry.id.clone();
        assert_eq!(holding(vec![entry]).way_to_open_as(&id, true), Some(Way::Open(gone)));
    }

    #[test]
    fn a_list_saved_before_bookmarks_should_load_and_need_the_picker_in_the_sandbox() {
        let temp = Temp::new("old-list");
        let a = temp.file("a.pdf");
        let file = temp.0.join("recent.json");
        let old = serde_json::json!({
            "version": 1,
            "entries": [{
                "id": "r0000000000000001", "path": a, "name": "a.pdf", "size": 8,
                "lastOpened": 3, "pages": 1, "fingerprint": null
            }]
        });
        std::fs::write(&file, old.to_string()).unwrap();
        let listed = RecentFiles::load(file).list_as(true);
        assert_eq!(
            listed
                .iter()
                .map(|e| (e.id.as_str(), e.has_handle, e.missing))
                .collect::<Vec<_>>(),
            vec![("r0000000000000001", false, false)]
        );
    }
}
