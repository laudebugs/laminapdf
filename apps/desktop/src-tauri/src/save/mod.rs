//! Saving a document's edits where it lives (E1 plan §7): the window's engine writes the save, and
//! the main process, which owns the file, puts the bytes in place.
//!
//! - **Save** appends the revision an incremental save writes to the file, or writes a full
//!   rewrite beside it and renames it over the file (`write.rs`). A save that would go over a file
//!   another app changed since the document read it is refused with `file-changed`, and the window
//!   asks what to do: reload it, save elsewhere, or overwrite it with a full rewrite.
//! - **Save as** writes a new file where the user chooses, which the document reads from then on.
//! - The watcher holds off while the app writes, so its own save never reloads the document.
//! - The document takes the save only once the file is written: a write that fails leaves the
//!   document unsaved and the file as it was.

mod write;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use laminapdf_desktop_protocol::{DocId, PlanReport, PlanSignature, SaveRequest, SaveWritten, SavedAs};
use serde::Serialize;
use tauri::Manager as _;
use tauri_plugin_dialog::DialogExt as _;

use crate::DesktopError;
use crate::engine::Engines;
use crate::watch::{FileStamp, Watcher};

/// A plan as the webview reads it: the fields of `@laminapdf/sdk`'s `PlannedSave` the desktop fills.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlanView {
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<SavedAs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refusal: Option<String>,
    constraint: laminapdf_desktop_protocol::PlanConstraint,
    signatures: Vec<SignatureView>,
    would_invalidate: Vec<SignatureView>,
    conformance_warnings: Vec<serde_json::Value>,
    unchanged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SignatureView {
    kind: laminapdf_desktop_protocol::SignatureRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<String>,
}

impl From<&PlanSignature> for SignatureView {
    fn from(signature: &PlanSignature) -> Self {
        Self {
            kind: signature.kind,
            field: signature.field.clone(),
        }
    }
}

impl From<PlanReport> for PlanView {
    fn from(plan: PlanReport) -> Self {
        Self {
            mode: plan.mode,
            refusal: plan.refusal,
            constraint: plan.constraint,
            signatures: plan.signatures.iter().map(SignatureView::from).collect(),
            would_invalidate: plan.would_invalidate.iter().map(SignatureView::from).collect(),
            // Each warning is its diagnostic's JSON; one the engine garbled is left out.
            conformance_warnings: plan
                .conformance_warnings
                .iter()
                .filter_map(|json| serde_json::from_str(json).ok())
                .collect(),
            unchanged: plan.unchanged,
        }
    }
}

/// What a save did, as the webview reads it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedView {
    /// The saved file's name, without its folder.
    name: String,
    mode: SavedAs,
    history_cleared: bool,
    metadata_not_updated: bool,
}

/// Where a save goes.
enum Target {
    /// Over the document's file. `overwrite` saves over a file another app changed.
    InPlace { overwrite: bool },
    /// A new file, which the document reads from then on.
    NewFile(PathBuf),
}

impl Target {
    /// The file the save writes, for a document whose file is `from`.
    fn path(&self, from: &Path) -> PathBuf {
        match self {
            Self::InPlace { .. } => from.to_owned(),
            Self::NewFile(path) => path.clone(),
        }
    }
}

/// Saves `window`'s document `doc` to `target`.
fn save(
    app: &tauri::AppHandle,
    window: &str,
    doc: DocId,
    mut request: SaveRequest,
    target: &Target,
) -> Result<SavedView, DesktopError> {
    let engines = Arc::clone(&app.state::<Arc<Engines>>());
    let from = engines.path_in(window, doc).ok_or(DesktopError::UnknownDocument)?;
    let to = target.path(&from);
    if engines.changed_on_disk(doc) {
        if matches!(target, Target::InPlace { overwrite: false }) {
            return Err(DesktopError::FileChanged);
        }
        // The file a revision would follow isn't the document's any more, so only a whole file
        // will do, and only while the document can still read the bytes it opened with.
        if !engines.base_intact(doc) {
            return Err(DesktopError::BaseGone);
        }
        request.rewrite = true;
    }
    // The file the save goes over, as it is now: it must still be so when the write happens.
    let expected = FileStamp::read(&to).ok();
    let written = engines.write_save(window, doc, request)?;
    let saving = Saving {
        app,
        engines: &engines,
        window,
        doc,
        from: &from,
        to: &to,
        expected,
    };
    if let Err(error) = saving.write(target, written) {
        engines.drop_save(window, doc);
        return Err(error);
    }
    if let Target::NewFile(_) = target {
        crate::opened::remember_saved(app, &to);
    }
    // A document that reopened from the saved file is watched under its new id already.
    saving.adopt(target)?;
    if let Target::NewFile(_) = target {
        // The document is the new file now: watch it there.
        let watcher = app.state::<Watcher>();
        watcher.unwatch(window, doc);
        watcher.watch(&to, window, doc);
    }
    Ok(SavedView {
        name: crate::opened::file_name(&to),
        mode: written.mode,
        history_cleared: written.history_cleared,
        metadata_not_updated: written.metadata_not_updated,
    })
}

/// One save under way: the document, the file it reads, the file the save goes to and how that
/// file looked when the save began.
struct Saving<'a> {
    app: &'a tauri::AppHandle,
    engines: &'a Engines,
    window: &'a str,
    doc: DocId,
    from: &'a Path,
    to: &'a Path,
    expected: Option<FileStamp>,
}

impl Saving<'_> {
    /// Writes the engine's output to the file, while the watcher holds off; the file is the
    /// document's from then on.
    fn write(&self, target: &Target, written: SaveWritten) -> Result<(), DesktopError> {
        let watcher = self.app.state::<Watcher>();
        let writing = watcher.writing(self.to, self.window, self.doc);
        let engines = self.engines;
        let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(self.window, self.doc, written.len, out);
        let mut unchanged = || self.unchanged();
        let result = match (target, written.mode) {
            (Target::InPlace { .. }, SavedAs::Incremental) => {
                // The revision follows the file as the document read it, and nothing else.
                let at = engines.file_len(self.doc).ok_or(DesktopError::UnknownDocument)?;
                unchanged().and_then(|()| write::append(self.to, at, &mut fill))
            }
            (Target::InPlace { .. }, SavedAs::FullRewrite) => write::replace(self.to, &mut fill, &mut unchanged),
            (Target::NewFile(_), SavedAs::Incremental) => {
                // The new file starts with the document's file, which must still be as it read it.
                if engines.changed_on_disk(self.doc) {
                    return Err(DesktopError::FileChanged);
                }
                write::create(self.to, Some(self.from), &mut fill)
            }
            (Target::NewFile(_), SavedAs::FullRewrite) => write::create(self.to, None, &mut fill),
        };
        result.map_err(|error| {
            if write::is_changed(&error) {
                DesktopError::FileChanged
            } else {
                DesktopError::Write(error)
            }
        })?;
        match FileStamp::read(self.to) {
            Ok(stamp) => writing.wrote(stamp),
            Err(error) => tracing::warn!("could not read a file just saved: {error}"),
        }
        Ok(())
    }

    /// `Ok` while the file the save goes over is as it was when the save began; else
    /// [`write::Changed`], which stops the write and [`Saving::write`] reports as `file-changed`.
    fn unchanged(&self) -> std::io::Result<()> {
        if FileStamp::read(self.to).ok() == self.expected {
            Ok(())
        } else {
            Err(std::io::Error::other(write::Changed))
        }
    }

    /// The file is written: the document reads it from now on, and the engine takes it as the
    /// saved file. The document's file source moves first, so the engine reads the grown file
    /// (on Windows the old source refuses it). If either step fails, the document reopens from
    /// the file it saved to, which holds every edit: `saved-reopened`. A Save as whose new file
    /// can't be opened at all leaves the document on its old file, which is unchanged.
    fn adopt(&self, target: &Target) -> Result<(), DesktopError> {
        if let Err(error) = self.engines.saved_to(self.doc, self.to) {
            tracing::warn!(doc = self.doc.0, "a saved file couldn't be opened again: {error}");
            self.engines.drop_save(self.window, self.doc);
            if let Target::NewFile(_) = target {
                return Err(DesktopError::Write(error));
            }
            crate::watch::app::reload_now(self.app, self.window, self.doc)?;
            return Err(DesktopError::SavedReopened);
        }
        if let Err(error) = self.engines.adopt_save(self.window, self.doc) {
            tracing::warn!(
                doc = self.doc.0,
                "the engine didn't adopt a saved file; reopening it: {error}"
            );
            crate::watch::app::reload_now(self.app, self.window, self.doc)?;
            return Err(DesktopError::SavedReopened);
        }
        Ok(())
    }
}

/// Runs `work` off the async runtime's threads: a save reads and writes files, and may wait for
/// a dialog.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, DesktopError> + Send + 'static,
) -> Result<T, DesktopError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| DesktopError::Internal(error.to_string()))?
}

/// How a save of this window's document `doc` would go.
#[tauri::command]
pub(crate) async fn plan_save(
    window: tauri::WebviewWindow,
    engines: tauri::State<'_, Arc<Engines>>,
    doc: u32,
    options: SaveRequest,
) -> Result<PlanView, DesktopError> {
    let engines = Arc::clone(&engines);
    blocking(move || Ok(engines.plan_save(window.label(), DocId(doc), options)?.into())).await
}

/// Saves this window's document `doc` over its file (File ▸ Save). With `overwrite`, saves over a
/// file another app changed, as a full rewrite; otherwise such a save fails with `file-changed`.
#[tauri::command]
pub(crate) async fn save_document(
    window: tauri::WebviewWindow,
    doc: u32,
    options: SaveRequest,
    overwrite: Option<bool>,
) -> Result<SavedView, DesktopError> {
    let target = Target::InPlace {
        overwrite: overwrite.unwrap_or(false),
    };
    blocking(move || save(window.app_handle(), window.label(), DocId(doc), options, &target)).await
}

/// Saves this window's document `doc` to a new file the user chooses (File ▸ Save as…), which
/// the document reads from then on. `None` when the dialog was cancelled.
#[tauri::command]
pub(crate) async fn save_document_as(
    window: tauri::WebviewWindow,
    doc: u32,
    options: SaveRequest,
) -> Result<Option<SavedView>, DesktopError> {
    blocking(move || {
        let app = window.app_handle();
        let from = app
            .state::<Arc<Engines>>()
            .path_in(window.label(), DocId(doc))
            .ok_or(DesktopError::UnknownDocument)?;
        let Some(to) = pick_save_path(&window, &from)? else {
            return Ok(None);
        };
        let target = if to == from {
            Target::InPlace { overwrite: true }
        } else {
            Target::NewFile(to)
        };
        save(app, window.label(), DocId(doc), options, &target).map(Some)
    })
    .await
}

/// Saves a copy of this window's document `doc` with the op log `log` applied to a new file the
/// user chooses (Save a copy of a change a signature refused): the copy is written whole, without
/// the signatures `options` lets it break, and `doc` stays as it was. Returns the copy's file name,
/// or `None` when the dialog was cancelled. The engine writes the copy and the main process puts it
/// in place, as a save does: it never goes through the webview.
#[tauri::command]
pub(crate) async fn save_copy_with(
    window: tauri::WebviewWindow,
    doc: u32,
    log: String,
    options: SaveRequest,
) -> Result<Option<String>, DesktopError> {
    blocking(move || {
        let app = window.app_handle();
        let engines = Arc::clone(&app.state::<Arc<Engines>>());
        let window_label = window.label();
        let doc = DocId(doc);
        let from = engines
            .path_in(window_label, doc)
            .ok_or(DesktopError::UnknownDocument)?;
        let Some(to) = pick_save_path(&window, &from)? else {
            return Ok(None);
        };
        // A file open in the app is never replaced behind its document's back.
        if to == from || engines.open_from(&to).is_some() {
            return Err(DesktopError::AlreadyOpen);
        }
        let request = SaveRequest {
            rewrite: true,
            ..options
        };
        let written = engines.write_copy_with(window_label, doc, log, request)?;
        let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(window_label, doc, written.len, out);
        let result = write::create(&to, None, &mut fill);
        engines.drop_save(window_label, doc);
        result.map_err(DesktopError::Write)?;
        // A copy or a merged file is not a document the person opened: it joins no recent list.
        Ok(Some(crate::opened::file_name(&to)))
    })
    .await
}

/// A file of a merge into a new document, as the webview names it.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MergeNewFile {
    /// The open document to take every page of.
    source: u32,
    /// The name of its bookmark.
    title: String,
}

/// Merges this window's document `doc`, as its edits leave it, and `files` after it (other
/// documents of the window) into a new file the user chooses (Merge documents… › Merge into a new
/// document), suggested the file name `suggested`. `doc` stays as it is. Returns the new file's name, or `None` when the dialog was
/// cancelled. The engine writes the new document and the main process puts it in place, as a save
/// does: it never goes through the webview.
#[tauri::command]
pub(crate) async fn merge_into_new_file(
    window: tauri::WebviewWindow,
    doc: u32,
    title: String,
    suggested: String,
    files: Vec<MergeNewFile>,
    outline: laminapdf_desktop_protocol::MergeOutline,
    fields: laminapdf_desktop_protocol::MergeFields,
) -> Result<Option<String>, DesktopError> {
    blocking(move || {
        let app = window.app_handle();
        let engines = Arc::clone(&app.state::<Arc<Engines>>());
        let window_label = window.label();
        let doc = DocId(doc);
        let from = engines
            .path_in(window_label, doc)
            .ok_or(DesktopError::UnknownDocument)?;
        let Some(to) = pick_new_file(&window, &suggested)? else {
            return Ok(None);
        };
        // A file open in the app is never replaced behind its document's back.
        if to == from || engines.open_from(&to).is_some() {
            return Err(DesktopError::AlreadyOpen);
        }
        let files: Vec<(DocId, String)> = files.into_iter().map(|file| (DocId(file.source), file.title)).collect();
        let written = engines.write_merge(window_label, doc, title, &files, (outline, fields))?;
        let mut fill = |out: &mut dyn std::io::Write| engines.copy_save(window_label, doc, written.len, out);
        let result = write::create(&to, None, &mut fill);
        engines.drop_save(window_label, doc);
        result.map_err(DesktopError::Write)?;
        // A copy or a merged file is not a document the person opened: it joins no recent list.
        Ok(Some(crate::opened::file_name(&to)))
    })
    .await
}

/// The file the user chooses to save `from` as, or `None` when they cancel. It blocks until they
/// answer: never on the main thread.
fn pick_save_path(window: &tauri::WebviewWindow, from: &Path) -> Result<Option<PathBuf>, DesktopError> {
    pick_new_file(window, &crate::opened::file_name(from))
}

/// The file the user chooses to save a PDF as, suggested `name`, or `None` when they cancel. It
/// blocks until they answer: never on the main thread.
fn pick_new_file(window: &tauri::WebviewWindow, name: &str) -> Result<Option<PathBuf>, DesktopError> {
    // End-to-end builds: a scenario's queued path stands in for the dialog.
    #[cfg(feature = "e2e")]
    if let Some(path) = window.state::<crate::e2e::QueuedSaveAs>().take() {
        return Ok(Some(path));
    }
    let mut dialog = window.dialog().file().set_parent(window).add_filter("PDF", &["pdf"]);
    if !name.is_empty() {
        dialog = dialog.set_file_name(name);
    }
    dialog
        .blocking_save_file()
        .map(tauri_plugin_dialog::FilePath::into_path)
        .transpose()
        .map_err(|error| DesktopError::Internal(error.to_string()))
}

/// Reloads this window's document `doc` from its file as it is on disk, dropping its unsaved
/// edits: the answer Reload to a file another app changed. The window hears `document://reloaded`
/// as for any reload.
#[tauri::command]
pub(crate) async fn reload_document(window: tauri::WebviewWindow, doc: u32) -> Result<(), DesktopError> {
    blocking(move || crate::watch::app::reload_now(window.app_handle(), window.label(), DocId(doc))).await
}
