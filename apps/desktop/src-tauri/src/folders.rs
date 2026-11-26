//! Writing files into a folder the person chose (Split document, Extract pages): the main process
//! keeps the folder's path and the webview names it by an id, so a path never reaches the webview
//! and a window reaches only the folders it chose. The engine writes each part of a document and
//! the main process puts it in the folder, as a save puts its output in place (`save/`): the
//! bytes never go through the webview.
//!
//! A part's name comes from the webview, so it is made safe first: one file name, with `.pdf`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use laminapdf_desktop_protocol::DocId;
use serde::{Deserialize, Serialize};
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;
use tauri_plugin_opener::OpenerExt as _;

use crate::DesktopError;
use crate::engine::Engines;
use crate::save::write;

/// The folders each window chose, by id.
#[derive(Debug, Default)]
pub(crate) struct Folders {
    next: AtomicU32,
    chosen: Mutex<BTreeMap<u32, (String, PathBuf)>>,
}

impl Folders {
    /// Remembers `path` as a folder of `window`, and gives its id.
    fn add(&self, window: &str, path: PathBuf) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.chosen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(id, (window.to_owned(), path));
        id
    }

    /// The path of folder `id`, which `window` chose.
    ///
    /// # Errors
    /// [`DesktopError::Internal`] for a folder another window chose, or none.
    fn path(&self, window: &str, id: u32) -> Result<PathBuf, DesktopError> {
        match self.chosen.lock().unwrap_or_else(PoisonError::into_inner).get(&id) {
            Some((owner, path)) if owner == window => Ok(path.clone()),
            _ => Err(DesktopError::Internal("no such folder in this window".to_owned())),
        }
    }
}

/// A folder as the webview knows it: its id, and its name as the person sees it.
#[derive(Debug, Serialize)]
pub(crate) struct FolderMeta {
    id: u32,
    name: String,
}

/// One part to write: its name, without `.pdf`, its pages' IDs, and whether it replaces a file of
/// its name that was there before the split began (else it takes a free name).
#[derive(Debug, Deserialize)]
pub(crate) struct PartFile {
    name: String,
    pages: Vec<String>,
    replace: bool,
}

/// The longest file name written, in bytes: well inside every file system's limit.
const MAX_NAME: usize = 180;

/// `name` as one file name ending in `.pdf`: no folder in it, no control characters, not empty.
///
/// # Errors
/// [`DesktopError::Internal`] for a name nothing safe is left of.
pub(crate) fn safe_name(name: &str) -> Result<String, DesktopError> {
    const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let trimmed = name.trim();
    // One `.pdf` is the extension we add; a name that already ends in it keeps one.
    let without = match trimmed.len().checked_sub(4).and_then(|at| trimmed.get(at..)) {
        Some(end) if end.eq_ignore_ascii_case(".pdf") => trimmed.get(..trimmed.len() - 4).unwrap_or(trimmed),
        _ => trimmed,
    };
    let stem = without.replace(|c: char| c.is_control() || FORBIDDEN.contains(&c), "_");
    let stem = stem.trim().trim_matches('.').trim();
    if stem.is_empty() {
        return Err(DesktopError::Internal("a part has no usable name".to_owned()));
    }
    // Windows reads CON.pdf, NUL.pdf, COM1.pdf … as devices, so such a name gets a letter before it.
    let reserved = stem
        .split('.')
        .next()
        .is_some_and(|first| is_reserved(first.trim_end()));
    let stem = if reserved { format!("_{stem}") } else { stem.to_owned() };
    let mut kept = String::new();
    for c in stem.chars() {
        if kept.len() + c.len_utf8() + ".pdf".len() > MAX_NAME {
            break;
        }
        kept.push(c);
    }
    Ok(format!("{kept}.pdf"))
}

/// Whether `stem` is a name Windows keeps for a device.
fn is_reserved(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            upper
                .strip_prefix(prefix)
                .is_some_and(|n| n.len() == 1 && n.chars().all(|c| ('1'..='9').contains(&c)))
        })
}

/// The first of `name`, `name (2).pdf`, `name (3).pdf` … that `taken` says is free in `folder`.
fn free_name(folder: &Path, name: &str, taken: &dyn Fn(&Path) -> bool) -> String {
    if !taken(&folder.join(name)) {
        return name.to_owned();
    }
    let stem = name.trim_end_matches(".pdf");
    // Ten thousand copies of one name is more than anyone keeps; the last resort keeps `name`.
    (2..10_000)
        .map(|n| format!("{stem} ({n}).pdf"))
        .find(|candidate| !taken(&folder.join(candidate)))
        .unwrap_or_else(|| name.to_owned())
}

/// Whether a file stands at `path`, or a document of the app is open from it.
fn in_use(engines: &Engines, path: &Path) -> bool {
    path.exists() || engines.open_from(path).is_some()
}

/// Runs `work` off the async runtime's threads: it reads and writes files, and may wait for a dialog.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, DesktopError> + Send + 'static,
) -> Result<T, DesktopError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Lets the user choose a folder to write files into, and keeps it for this window; `None` when
/// they cancel. The dialog runs from Rust, so the path never reaches the webview.
#[tauri::command]
pub(crate) async fn choose_folder(window: tauri::WebviewWindow) -> Result<Option<FolderMeta>, DesktopError> {
    blocking(move || {
        let Some(path) = pick_folder(&window)? else {
            return Ok(None);
        };
        let name = crate::opened::file_name(&path);
        let id = window.app_handle().state::<Folders>().add(window.label(), path);
        Ok(Some(FolderMeta { id, name }))
    })
    .await
}

/// The folder the user picks over `window`, or `None` when they cancel. It blocks until they
/// answer: never on the main thread.
fn pick_folder(window: &tauri::WebviewWindow) -> Result<Option<PathBuf>, DesktopError> {
    // End-to-end builds: a scenario's queued folder stands in for the dialog.
    #[cfg(feature = "e2e")]
    if let Some(path) = window.state::<crate::e2e::QueuedFolder>().take() {
        return Ok(Some(path));
    }
    window
        .dialog()
        .file()
        .set_parent(window)
        .blocking_pick_folder()
        .map(tauri_plugin_dialog::FilePath::into_path)
        .transpose()
        .map_err(|error| DesktopError::Internal(error.to_string()))
}

/// Which of `names` are files in this window's folder `folder` already, as it would write them.
#[tauri::command]
pub(crate) async fn folder_existing(
    window: tauri::WebviewWindow,
    folders: tauri::State<'_, Folders>,
    engines: tauri::State<'_, Arc<Engines>>,
    folder: u32,
    names: Vec<String>,
) -> Result<Vec<String>, DesktopError> {
    let path = folders.path(window.label(), folder)?;
    let engines = Arc::clone(&engines);
    blocking(move || {
        let mut taken = Vec::new();
        for name in names {
            if in_use(&engines, &path.join(safe_name(&name)?)) {
                taken.push(name);
            }
        }
        Ok(taken)
    })
    .await
}

/// Writes `bytes` to the file `name` in this window's folder `folder`, replacing a file of that name.
#[tauri::command]
pub(crate) async fn folder_write(
    window: tauri::WebviewWindow,
    folders: tauri::State<'_, Folders>,
    engines: tauri::State<'_, Arc<Engines>>,
    folder: u32,
    name: String,
    bytes: Vec<u8>,
) -> Result<(), DesktopError> {
    let path = folders.path(window.label(), folder)?.join(safe_name(&name)?);
    let engines = Arc::clone(&engines);
    blocking(move || {
        // A file open in the app is never replaced behind its document's back.
        if engines.open_from(&path).is_some() {
            return Err(DesktopError::AlreadyOpen);
        }
        let mut fill = |out: &mut dyn std::io::Write| out.write_all(&bytes);
        write::create(&path, None, &mut fill).map_err(DesktopError::Write)
    })
    .await
}

/// Writes each of `parts`, new documents of pages of this window's document `doc` as its edits
/// leave it, into this window's folder `folder`: a part that `replace`s takes the name of the file
/// that was there, else it gets a free name (so two parts of one name, even in another case on
/// a file system that ignores it, never replace each other). A file open in the app is never
/// replaced. Returns the names written, in order. A part that fails stops the rest; the files
/// before it stay.
#[tauri::command]
pub(crate) async fn write_parts(
    window: tauri::WebviewWindow,
    folders: tauri::State<'_, Folders>,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    folder: u32,
    parts: Vec<PartFile>,
) -> Result<Vec<String>, DesktopError> {
    let path = folders.path(window.label(), folder)?;
    let engines = Arc::clone(&engines);
    blocking(move || {
        let label = window.label();
        let doc = DocId(doc);
        let mut written = Vec::with_capacity(parts.len());
        for part in parts {
            let safe = safe_name(&part.name)?;
            let name = if part.replace {
                if engines.open_from(&path.join(&safe)).is_some() {
                    return Err(DesktopError::AlreadyOpen);
                }
                safe
            } else {
                free_name(&path, &safe, &|candidate| in_use(&engines, candidate))
            };
            let part_written = engines.write_part(label, doc, part.pages)?;
            let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(label, doc, part_written.len, out);
            let result = write::create(&path.join(&name), None, &mut fill);
            engines.drop_save(label, doc);
            result.map_err(DesktopError::Write)?;
            written.push(name);
        }
        Ok(written)
    })
    .await
}

/// Writes one part, a new document of `pages` of this window's document `doc`, to a file the user
/// chooses, suggested the name `suggested` (Extract pages, and a split into one file). Returns the
/// file's name, or `None` when the dialog was cancelled.
#[tauri::command]
pub(crate) async fn save_part(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    suggested: String,
    pages: Vec<String>,
) -> Result<Option<String>, DesktopError> {
    let engines = Arc::clone(&engines);
    blocking(move || {
        let label = window.label();
        let doc = DocId(doc);
        let from = engines.path_in(label, doc).ok_or(DesktopError::UnknownDocument)?;
        let Some(to) = crate::save::pick_new_file(&window, &suggested)? else {
            return Ok(None);
        };
        // A file open in the app is never replaced behind its document's back.
        if to == from || engines.open_from(&to).is_some() {
            return Err(DesktopError::AlreadyOpen);
        }
        let written = engines.write_part(label, doc, pages)?;
        let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(label, doc, written.len, out);
        let result = write::create(&to, None, &mut fill);
        engines.drop_save(label, doc);
        result.map_err(DesktopError::Write)?;
        Ok(Some(crate::opened::file_name(&to)))
    })
    .await
}

/// Shows file `name` of this window's folder `folder` in Finder or Explorer.
#[tauri::command]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its window, state and arguments by value"
)]
pub(crate) fn reveal_in_folder(
    window: tauri::WebviewWindow,
    folders: tauri::State<'_, Folders>,
    folder: u32,
    name: String,
) -> Result<(), DesktopError> {
    let path = folders.path(window.label(), folder)?.join(safe_name(&name)?);
    window
        .app_handle()
        .opener()
        .reveal_item_in_dir(path)
        .map_err(|error| DesktopError::Internal(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_should_become_one_file_name_ending_in_pdf() {
        let cases = [
            ("part 1", "part 1.pdf"),
            ("part 1.pdf", "part 1.pdf"),
            ("../../etc/passwd", "_.._etc_passwd.pdf"),
            ("a\\b:c", "a_b_c.pdf"),
            ("  .hidden  ", "hidden.pdf"),
        ];
        for (name, safe) in cases {
            assert_eq!(safe_name(name).ok().as_deref(), Some(safe), "{name}");
        }
    }

    #[test]
    fn a_name_windows_keeps_for_a_device_should_get_a_letter_before_it() {
        let cases = [
            ("CON", "_CON.pdf"),
            ("nul.txt", "_nul.txt.pdf"),
            ("com1", "_com1.pdf"),
            ("com10", "com10.pdf"),
        ];
        for (name, safe) in cases {
            assert_eq!(safe_name(name).ok().as_deref(), Some(safe), "{name}");
        }
    }

    #[test]
    fn only_one_pdf_extension_should_be_taken_off() {
        assert_eq!(safe_name("a.pdf.pdf").ok().as_deref(), Some("a.pdf.pdf"));
    }

    #[test]
    fn a_name_with_nothing_usable_should_be_refused() {
        assert!(safe_name("").is_err() && safe_name("   ").is_err() && safe_name("...").is_err());
    }

    #[test]
    fn a_long_name_should_be_cut_to_fit() {
        let safe = safe_name(&"é".repeat(300)).unwrap_or_default();
        assert!(safe.len() <= MAX_NAME && Path::new(&safe).extension().is_some_and(|e| e == "pdf"));
    }

    #[test]
    fn a_taken_name_should_get_the_next_free_one() {
        let dir = std::env::temp_dir().join(format!("folders-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_default();
        std::fs::write(dir.join("a.pdf"), b"x").unwrap_or_default();
        std::fs::write(dir.join("a (2).pdf"), b"x").unwrap_or_default();
        let none = |path: &Path| path.exists();
        let (free, kept) = (free_name(&dir, "a.pdf", &none), free_name(&dir, "b.pdf", &none));
        std::fs::remove_dir_all(&dir).unwrap_or_default();
        assert_eq!((free.as_str(), kept.as_str()), ("a (3).pdf", "b.pdf"));
    }

    #[test]
    fn a_window_should_reach_only_the_folders_it_chose() {
        let folders = Folders::default();
        let id = folders.add("main", PathBuf::from("/tmp/x"));
        assert!(folders.path("main", id).is_ok() && folders.path("doc-2", id).is_err());
        assert!(folders.path("main", id + 1).is_err());
    }
}
