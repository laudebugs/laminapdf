//! The watcher's way into the app: a changed file reopens in its window's engine and the window is
//! told, by the events below, sent to that window only. The webview names documents by id; no
//! path is in any payload.

use std::path::Path;
use std::sync::Arc;

use laminapdf_desktop_protocol::DocId;
use serde::Serialize;
use tauri::{Emitter as _, EventTarget, Manager as _};

use super::{Reload, Reloaded};
use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines};
use crate::prefs::Prefs;
use crate::unsaved::Unsaved;

/// The window's document reopened from its changed file, under a new id; the old one is closed.
pub(crate) const RELOADED_EVENT: &str = "document://reloaded";
/// The window's document's file is gone; the document stays readable from what is open.
pub(crate) const MISSING_EVENT: &str = "document://missing";
/// The window's document's file changed but couldn't be opened; the old view stays.
pub(crate) const UNREADABLE_EVENT: &str = "document://unreadable";
/// Another app changed the window's document's file while the document held unsaved edits: the
/// window asks whether to reload, save elsewhere or overwrite, so neither change is lost unasked.
pub(crate) const CONFLICT_EVENT: &str = "document://conflict";

/// [`RELOADED_EVENT`]'s payload.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ReloadedPayload {
    /// The id the window knew the document by.
    pub previous: u32,
    pub document: DocumentMeta,
}

/// [`MISSING_EVENT`]'s and [`UNREADABLE_EVENT`]'s payload.
#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) struct DocPayload {
    pub doc: u32,
}

/// Reloads through the app's engines and tells the window.
pub(crate) struct WindowReload(pub tauri::AppHandle);

impl WindowReload {
    fn tell<P: Serialize + Clone>(&self, window: &str, event: &str, payload: P) {
        tell(&self.0, window, event, payload);
    }
}

fn tell<P: Serialize + Clone>(app: &tauri::AppHandle, window: &str, event: &str, payload: P) {
    if let Err(error) = app.emit_to(EventTarget::webview_window(window), event, payload) {
        tracing::warn!("could not tell a window its file changed: {error}");
    }
}

/// Reopens `window`'s document `doc` from `path` and tells the window: the new document's id, or
/// the failure, which leaves the old view and tells the window the file is unreadable.
fn reload(app: &tauri::AppHandle, window: &str, doc: DocId, path: &Path) -> Result<DocId, DesktopError> {
    match app.state::<Arc<Engines>>().reload(window, doc, path) {
        Ok(document) => {
            let new = DocId(document.doc);
            tell(
                app,
                window,
                RELOADED_EVENT,
                ReloadedPayload {
                    previous: doc.0,
                    document,
                },
            );
            Ok(new)
        }
        Err(error) => {
            tracing::warn!(
                window,
                doc = doc.0,
                "a changed file didn't reopen; keeping the old view: {error}"
            );
            tell(app, window, UNREADABLE_EVENT, DocPayload { doc: doc.0 });
            Err(error)
        }
    }
}

/// Reloads `window`'s document `doc` from its file now, whatever its unsaved edits: the user chose
/// Reload. The watcher follows the document under its new id.
///
/// # Errors
/// `unknown-document`; the reload's failure, after which the old view stays.
pub(crate) fn reload_now(app: &tauri::AppHandle, window: &str, doc: DocId) -> Result<(), DesktopError> {
    let path = app
        .state::<Arc<Engines>>()
        .path_in(window, doc)
        .ok_or(DesktopError::UnknownDocument)?;
    let new = reload(app, window, doc, &path)?;
    let watcher = app.state::<crate::watch::Watcher>();
    watcher.unwatch(window, doc);
    watcher.watch(&path, window, new);
    Ok(())
}

impl Reload for WindowReload {
    fn modified(&self, window: &str, doc: DocId, path: &Path) -> Reloaded {
        if self.0.state::<Unsaved>().holds_doc(window, doc.0) {
            tracing::info!(window, doc = doc.0, "a document with unsaved edits changed on disk");
            self.tell(window, CONFLICT_EVENT, DocPayload { doc: doc.0 });
            return Reloaded::Skipped;
        }
        if !self.0.state::<Prefs>().get().reload_on_change {
            return Reloaded::Skipped;
        }
        match reload(&self.0, window, doc, path) {
            Ok(new) => Reloaded::Replaced(new),
            Err(_) => Reloaded::Failed,
        }
    }

    fn missing(&self, window: &str, doc: DocId) {
        tracing::info!(window, doc = doc.0, "an open document's file was moved or deleted");
        self.tell(window, MISSING_EVENT, DocPayload { doc: doc.0 });
    }
}
