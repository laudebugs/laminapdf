//! What happens around a document that opens from a path, whoever asked (the open dialog, the
//! OS, the recent list): its window's engine opens it, the app's and the OS's recent lists record
//! it (unless the user turned that off), and the watcher watches its file. Closing undoes the watch.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use laminapdf_desktop_protocol::DocId;
use sha2::{Digest as _, Sha256};
use tauri::Manager as _;

use crate::DesktopError;
use crate::engine::{DocumentMeta, Engines};
use crate::prefs::Prefs;
use crate::recent::{self, RecentFiles};
use crate::watch::Watcher;

/// Opens `path` in `window`, then records and watches it.
pub(crate) fn open_path(app: &tauri::AppHandle, window: &str, path: &Path) -> Result<DocumentMeta, DesktopError> {
    let opened = app.state::<Arc<Engines>>().open(window, path)?;
    remember(app, path, &opened);
    app.state::<Watcher>().watch(path, window, DocId(opened.doc));
    Ok(opened)
}

/// `window`'s document `doc` closed: its file is no longer watched for it.
pub(crate) fn closed(app: &tauri::AppHandle, window: &str, doc: DocId) {
    app.state::<Watcher>().unwatch(window, doc);
}

/// Puts the opened file first in the app's recent list and adds it to the OS's.
fn remember(app: &tauri::AppHandle, path: &Path, opened: &DocumentMeta) {
    if !app.state::<Prefs>().get().remember_recent {
        return;
    }
    app.state::<RecentFiles>().add(
        recent::Opened {
            path,
            name: &opened.name,
            size: opened.size,
            pages: (!opened.needs_password).then_some(opened.page_count),
            fingerprint: fingerprint(path),
        },
        recent::now_ms(),
    );
    let path = path.to_owned();
    on_main_thread(app, move |_| laminapdf_desktop_os::note_recent(&path));
}

/// Removes `paths` from the OS's recent list: every one of them on Linux, the app's whole list on
/// macOS and Windows when `all`, else each path where the OS can remove one.
pub(crate) fn forget(app: &tauri::AppHandle, paths: Vec<PathBuf>, all: bool) {
    on_main_thread(app, move |app_id| {
        if all {
            laminapdf_desktop_os::clear_recent(app_id, &paths)
        } else {
            paths
                .iter()
                .try_for_each(|path| laminapdf_desktop_os::forget_recent(app_id, path))
        }
    });
}

/// Runs an OS recent-list call on the main thread (`AppKit`'s and GTK's rule), with the app's ID;
/// a refusal is logged, never shown: the app's own list is what the user manages.
fn on_main_thread(
    app: &tauri::AppHandle,
    call: impl FnOnce(&str) -> Result<(), laminapdf_desktop_os::OsError> + Send + 'static,
) {
    let app_id = app.config().identifier.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        if let Err(error) = call(&app_id) {
            tracing::warn!("the OS's recent list refused a change: {error}");
        }
    }) {
        tracing::warn!("could not reach the main thread: {error}");
    }
}

/// The document's fingerprint, the key its reading position is kept under: `"fp1:"` and the
/// SHA-256 of a tag and the file's head through its first `%%EOF` within 64 KiB, as the viewer's
/// worker computes it (packages/viewer/src/worker/fingerprint.ts). Hashing reads bytes; it parses
/// nothing (ADR 0008 §2).
pub(crate) fn fingerprint(path: &Path) -> Option<String> {
    let mut head = Vec::new();
    File::open(path).ok()?.take(64 * 1024).read_to_end(&mut head).ok()?;
    Some(fingerprint_of(&head))
}

fn fingerprint_of(head: &[u8]) -> String {
    const EOF: &[u8] = b"%%EOF";
    let covered = head
        .windows(EOF.len())
        .position(|window| window == EOF)
        .and_then(|at| head.get(..at + EOF.len()))
        .unwrap_or(head);
    let mut hasher = Sha256::new();
    hasher.update(b"laminapdf.fp/1\0");
    hasher.update(covered);
    let digest = hasher.finalize();
    digest.iter().fold(String::from("fp1:"), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::fingerprint_of;

    #[test]
    fn the_fingerprint_should_cover_the_head_through_its_first_eof() {
        let first = fingerprint_of(b"%PDF-1.7\nfirst\n%%EOF\n");
        let appended = fingerprint_of(b"%PDF-1.7\nfirst\n%%EOF\nan update\n%%EOF\n");
        assert_eq!(first, appended);
    }

    #[test]
    fn a_different_head_should_fingerprint_differently() {
        assert_ne!(
            fingerprint_of(b"%PDF-1.7\na\n%%EOF"),
            fingerprint_of(b"%PDF-1.7\nb\n%%EOF")
        );
    }

    /// The viewer's `documentFingerprint` of the same bytes (packages/viewer/src/worker/
    /// fingerprint.ts), so a recent entry finds the reading position the viewer keeps.
    #[test]
    fn the_fingerprint_should_be_the_viewers() {
        assert_eq!(
            fingerprint_of(b"%PDF-1.7\nfirst\n%%EOF\nan update\n%%EOF\n"),
            "fp1:89659a037fa63ca431cac392490cce9c4ddc02a74e02fdf3611629cd92ff7477"
        );
    }
}
