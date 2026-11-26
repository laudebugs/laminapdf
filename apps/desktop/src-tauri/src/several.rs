//! Several PDFs opened together (Paper O07; E2u): a command line, a second launch, Finder or a drop
//! that names two or more files. The window asks what to do with them, as the web reader does:
//! merge them into one new document, or open each in its own tab. The main process keeps the
//! files' paths under a request id and tells the webview only their names (ADR 0008 §6); the
//! webview answers with the id.
//!
//! A merge opens each file in the window's engine as a document of its own, has the engine write
//! the new document (the first file's pages, then each of the others'), and writes it to a file the
//! person chooses, which then opens in a tab. A file that doesn't open, or is encrypted, stops the
//! merge with the request still waiting, so the person can open the files one at a time instead.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use laminapdf_desktop_protocol::{DocId, MergeFields, MergeOutline};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _, EventTarget, Manager as _};

use crate::DesktopError;
use crate::engine::Engines;
use crate::save::write;

/// What the webview hears when a request names several files: [`Asked`]. Repeated in `src/several.tsx`.
pub(crate) const SEVERAL_EVENT: &str = "open://several";

/// The requests waiting for an answer, by id: the window that asked and the files, in order.
#[derive(Debug, Default)]
pub(crate) struct Several {
    next: AtomicU32,
    waiting: Mutex<BTreeMap<u32, (String, Vec<PathBuf>)>>,
}

impl Several {
    fn waiting(&self) -> std::sync::MutexGuard<'_, BTreeMap<u32, (String, Vec<PathBuf>)>> {
        // A map of paths: no invariant a panic could break halfway.
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn add(&self, window: &str, paths: Vec<PathBuf>) -> u32 {
        let id = self.next.fetch_add(1, Ordering::Relaxed) + 1;
        self.waiting().insert(id, (window.to_owned(), paths));
        id
    }

    /// Takes request `id` out, when `window` asked it: whoever holds the files is answering it, and
    /// a second answer finds nothing. [`Self::put_back`] returns them when the answer didn't finish it.
    fn claim(&self, window: &str, id: u32) -> Option<Vec<PathBuf>> {
        let mut waiting = self.waiting();
        match waiting.get(&id) {
            Some((owner, _)) if owner == window => waiting.remove(&id).map(|(_, paths)| paths),
            _ => None,
        }
    }

    /// Puts the files of a claimed request back, still waiting for an answer.
    fn put_back(&self, window: &str, id: u32, paths: Vec<PathBuf>) {
        self.waiting().insert(id, (window.to_owned(), paths));
    }

    /// Takes request `id` out: it is answered.
    fn finish(&self, id: u32) {
        self.waiting().remove(&id);
    }

    /// `window` closed: its requests go.
    pub(crate) fn closed(&self, window: &str) {
        self.waiting().retain(|_, (owner, _)| owner != window);
    }
}

/// What [`SEVERAL_EVENT`] carries: the request, and the files' names (never their folders).
#[derive(Debug, Clone, Serialize)]
struct Asked {
    request: u32,
    names: Vec<String>,
}

/// Asks `window` what to do with `paths`, which a request named together. When the window can't be
/// told, the files open one by one, as they would have without the question.
pub(crate) fn ask(app: &AppHandle, window: &str, paths: Vec<PathBuf>) {
    let names = paths.iter().map(|path| crate::opened::file_name(path)).collect();
    let request = app.state::<Several>().add(window, paths.clone());
    let asked = Asked { request, names };
    if let Err(error) = app.emit_to(EventTarget::webview_window(window), SEVERAL_EVENT, asked) {
        tracing::warn!("could not ask the window about several files: {error}");
        app.state::<Several>().finish(request);
        crate::open::open_each(app, window, paths);
    }
}

/// What the person chose.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Answer {
    /// Merge them into one new file, which opens.
    Merge,
    /// Open each in its own tab.
    Each,
    /// Open none.
    Cancel,
}

/// Answers request `request`: opens each file, merges them (suggesting the file name `suggested`),
/// or drops the request. A merge that fails leaves the request waiting; one the person cancels in
/// the save dialog does too, so they can still choose. Resolves to whether the request is done.
#[tauri::command]
pub(crate) async fn answer_several(
    window: tauri::WebviewWindow,
    request: u32,
    answer: Answer,
    suggested: Option<String>,
) -> Result<bool, DesktopError> {
    tauri::async_runtime::spawn_blocking(move || {
        let app = window.app_handle();
        let several = app.state::<Several>();
        let label = window.label();
        let paths = several.claim(label, request).ok_or(DesktopError::UnknownDocument)?;
        match answer {
            Answer::Cancel => {}
            Answer::Each => crate::open::open_each(app, label, paths),
            Answer::Merge => {
                let merged = suggested
                    .ok_or_else(|| DesktopError::Internal("a merge needs a suggested name".to_owned()))
                    .and_then(|name| merge(&window, &paths, &name));
                match merged {
                    Ok(Some(merged)) => crate::open::open_each(app, label, vec![merged]),
                    // Cancelled, or failed: the question waits for another choice.
                    other => {
                        several.put_back(label, request, paths);
                        return other.map(|_| false);
                    }
                }
            }
        }
        Ok(true)
    })
    .await
    .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// Merges the files at `paths` into a new file the person chooses, suggested the name `suggested`,
/// and gives its path; `None` when they cancel the dialog. The files open as documents of the
/// window for the time it takes, and close again.
fn merge(window: &tauri::WebviewWindow, paths: &[PathBuf], suggested: &str) -> Result<Option<PathBuf>, DesktopError> {
    let app = window.app_handle();
    let engines = Arc::clone(&app.state::<Arc<Engines>>());
    let label = window.label();
    let mut opened = Opened {
        engines: &engines,
        window: label,
        docs: Vec::with_capacity(paths.len()),
    };
    for path in paths {
        let meta = engines.open(label, path)?;
        let doc = DocId(meta.doc);
        opened.docs.push((doc, stem(path)));
        // An encrypted file would need its password asked first: the merge stops there.
        if meta.needs_password {
            return Err(DesktopError::Internal("a file needs its password".to_owned()));
        }
    }
    write_merged(window, &engines, &opened.docs, suggested)
}

/// The documents a merge opened for its files: closed again when it ends, however it ends.
struct Opened<'a> {
    engines: &'a Engines,
    window: &'a str,
    docs: Vec<(DocId, String)>,
}

impl Drop for Opened<'_> {
    fn drop(&mut self) {
        for (doc, _) in &self.docs {
            // Each was opened here; a failure to close one leaves it for the window's close.
            if let Err(error) = self.engines.close(self.window, *doc) {
                tracing::debug!("could not close a file opened for a merge: {error}");
            }
        }
    }
}

/// The file `path` names, without its folder and `.pdf`: its bookmark's name.
fn stem(path: &std::path::Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// Asks where the merged document goes, has the engine write it, and puts it there.
fn write_merged(
    window: &tauri::WebviewWindow,
    engines: &Engines,
    opened: &[(DocId, String)],
    suggested: &str,
) -> Result<Option<PathBuf>, DesktopError> {
    let label = window.label();
    let Some(to) = crate::save::pick_new_file(window, suggested)? else {
        return Ok(None);
    };
    // A file that one of these is open from is never replaced.
    if engines.open_from(&to).is_some() {
        return Err(DesktopError::AlreadyOpen);
    }
    let Some(((first, title), rest)) = opened.split_first() else {
        return Ok(None);
    };
    let written = engines.write_merge(
        label,
        *first,
        title.clone(),
        rest,
        (MergeOutline::PerFile, MergeFields::Rename),
    )?;
    let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(label, *first, written.len, out);
    let result = write::create(&to, None, &mut fill);
    engines.drop_save(label, *first);
    result.map_err(DesktopError::Write)?;
    Ok(Some(to))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> Vec<PathBuf> {
        vec![PathBuf::from("/a/one.pdf"), PathBuf::from("/a/two.pdf")]
    }

    #[test]
    fn a_window_should_reach_only_the_request_it_asked() {
        let several = Several::default();
        let id = several.add("main", paths());
        assert_eq!(several.claim("doc-2", id), None);
        assert_eq!(several.claim("main", id + 1), None);
        assert_eq!(several.claim("main", id), Some(paths()));
    }

    #[test]
    fn an_answered_request_should_be_gone() {
        let several = Several::default();
        let id = several.add("main", paths());
        several.finish(id);
        assert_eq!(several.claim("main", id), None);
    }

    #[test]
    fn a_closed_window_should_take_its_requests_with_it() {
        let several = Several::default();
        let (mine, other) = (several.add("doc-1", paths()), several.add("main", paths()));
        several.closed("doc-1");
        assert_eq!(
            (several.claim("doc-1", mine), several.claim("main", other)),
            (None, Some(paths()))
        );
    }

    #[test]
    fn a_request_should_be_answered_once_and_wait_again_when_put_back() {
        let several = Several::default();
        let id = several.add("main", paths());
        let claimed = several.claim("main", id);
        // A second answer meanwhile finds nothing.
        assert_eq!((several.claim("main", id), claimed.is_some()), (None, true));
        several.put_back("main", id, paths());
        assert_eq!(several.claim("main", id), Some(paths()));
    }

    #[test]
    fn two_requests_should_get_two_ids() {
        let several = Several::default();
        assert_ne!(several.add("main", paths()), several.add("main", paths()));
    }
}
