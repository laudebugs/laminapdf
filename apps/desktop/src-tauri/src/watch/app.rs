//! The watcher's way into the app: a changed file reopens in its window's engine and the window is
//! told, by the events below, sent to that window only. The webview names documents by id; no
//! path is in any payload.

use std::path::Path;
use std::sync::Arc;

use laminapdf_desktop_protocol::DocId;
use serde::Serialize;
use tauri::{Emitter as _, EventTarget, Manager as _};

use super::{Reload, Reloaded};
use crate::engine::{DocumentMeta, Engines};
use crate::prefs::Prefs;

/// The window's document reopened from its changed file, under a new id; the old one is closed.
pub(crate) const RELOADED_EVENT: &str = "document://reloaded";
/// The window's document's file is gone; the document stays readable from what is open.
pub(crate) const MISSING_EVENT: &str = "document://missing";
/// The window's document's file changed but couldn't be opened; the old view stays.
pub(crate) const UNREADABLE_EVENT: &str = "document://unreadable";

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
        if let Err(error) = self.0.emit_to(EventTarget::webview_window(window), event, payload) {
            tracing::warn!("could not tell a window its file changed: {error}");
        }
    }
}

impl Reload for WindowReload {
    fn modified(&self, window: &str, doc: DocId, path: &Path) -> Reloaded {
        if !self.0.state::<Prefs>().get().reload_on_change {
            return Reloaded::Skipped;
        }
        match self.0.state::<Arc<Engines>>().reload(window, doc, path) {
            Ok(document) => {
                let new = DocId(document.doc);
                self.tell(
                    window,
                    RELOADED_EVENT,
                    ReloadedPayload {
                        previous: doc.0,
                        document,
                    },
                );
                Reloaded::Replaced(new)
            }
            Err(error) => {
                tracing::warn!(
                    window,
                    doc = doc.0,
                    "a changed file didn't reopen; keeping the old view: {error}"
                );
                self.tell(window, UNREADABLE_EVENT, DocPayload { doc: doc.0 });
                Reloaded::Failed
            }
        }
    }

    fn missing(&self, window: &str, doc: DocId) {
        tracing::info!(window, doc = doc.0, "an open document's file was moved or deleted");
        self.tell(window, MISSING_EVENT, DocPayload { doc: doc.0 });
    }
}
