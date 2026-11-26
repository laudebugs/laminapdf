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

/// One part to write: its name, without `.pdf`, and its pages' IDs.
#[derive(Debug, Deserialize)]
pub(crate) struct PartFile {
    name: String,
    pages: Vec<String>,
}

/// The longest file name written, in bytes: well inside every file system's limit.
const MAX_NAME: usize = 180;

/// `name` as one file name ending in `.pdf`: no folder in it, no control characters, not empty.
///
/// # Errors
/// [`DesktopError::Internal`] for a name nothing safe is left of.
pub(crate) fn safe_name(name: &str) -> Result<String, DesktopError> {
    const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    let stem = name
        .trim()
        .trim_end_matches(".pdf")
        .trim_end_matches(".PDF")
        .replace(|c: char| c.is_control() || FORBIDDEN.contains(&c), "_");
    let stem = stem.trim().trim_matches('.').trim();
    if stem.is_empty() {
        return Err(DesktopError::Internal("a part has no usable name".to_owned()));
    }
    let mut kept = String::new();
    for c in stem.chars() {
        if kept.len() + c.len_utf8() + ".pdf".len() > MAX_NAME {
            break;
        }
        kept.push(c);
    }
    Ok(format!("{kept}.pdf"))
}

/// The first of `name`, `name (2).pdf`, `name (3).pdf` … not taken in `folder`.
fn free_name(folder: &Path, name: &str) -> String {
    if !folder.join(name).exists() {
        return name.to_owned();
    }
    let stem = name.trim_end_matches(".pdf");
    // Ten thousand copies of one name is more than anyone keeps; the last resort replaces the file.
    (2..10_000)
        .map(|n| format!("{stem} ({n}).pdf"))
        .find(|candidate| !folder.join(candidate).exists())
        .unwrap_or_else(|| name.to_owned())
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
    folder: u32,
    names: Vec<String>,
) -> Result<Vec<String>, DesktopError> {
    let path = folders.path(window.label(), folder)?;
    blocking(move || {
        let mut taken = Vec::new();
        for name in names {
            let safe = safe_name(&name)?;
            if path.join(&safe).exists() {
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
    folder: u32,
    name: String,
    bytes: Vec<u8>,
) -> Result<(), DesktopError> {
    let path = folders.path(window.label(), folder)?.join(safe_name(&name)?);
    blocking(move || {
        let mut fill = |out: &mut dyn std::io::Write| out.write_all(&bytes);
        write::create(&path, None, &mut fill).map_err(DesktopError::Write)
    })
    .await
}

/// Writes each of `parts`, new documents of pages of this window's document `doc` as its edits
/// leave it, into this window's folder `folder`: a file whose name is taken is replaced when
/// `replace`, else the part gets a free name. Returns the names written, in order. A part that
/// fails stops the rest; the files before it stay.
#[tauri::command]
pub(crate) async fn write_parts(
    window: tauri::WebviewWindow,
    folders: tauri::State<'_, Folders>,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    folder: u32,
    parts: Vec<PartFile>,
    replace: bool,
) -> Result<Vec<String>, DesktopError> {
    let path = folders.path(window.label(), folder)?;
    let engines = Arc::clone(&engines);
    blocking(move || {
        let label = window.label();
        let doc = DocId(doc);
        let mut written = Vec::with_capacity(parts.len());
        for part in parts {
            let safe = safe_name(&part.name)?;
            let name = if replace { safe } else { free_name(&path, &safe) };
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
        let (free, kept) = (free_name(&dir, "a.pdf"), free_name(&dir, "b.pdf"));
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
