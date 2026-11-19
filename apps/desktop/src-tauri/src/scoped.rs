//! Files found again from their bookmarks. In the Mac App Store build the app runs in App
//! Sandbox (ADR 0014 §11): a file the user opened earlier is reachable again only through its
//! security-scoped bookmark, and only while the access that resolving it starts lasts. This keeps
//! that access, one per file, for as long as the app runs: the files are at most the recent list
//! and the session's tabs. Elsewhere bookmarks are plain and there is no access to keep.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use laminapdf_desktop_os::{FileAccess, file_bookmark, resolve_bookmark};

use crate::session::{from_hex, to_hex};

/// The access each found file holds, by its path.
static HELD: Mutex<BTreeMap<PathBuf, FileAccess>> = Mutex::new(BTreeMap::new());

/// Where the file `bookmark` names is now, kept reachable; `None` when it can't be found.
pub(crate) fn find(bookmark: &[u8]) -> Option<PathBuf> {
    let (path, access) = resolve_bookmark(bookmark)?;
    HELD.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.clone(), access);
    Some(path)
}

/// [`find`] for a bookmark kept as hex.
pub(crate) fn find_hex(bookmark: &str) -> Option<PathBuf> {
    find(&from_hex(bookmark)?)
}

/// A bookmark of `path`, as hex, when the store build needs one to reach the file again; `None`
/// in the direct download, which keeps paths.
pub(crate) fn bookmark_hex(path: &Path) -> Option<String> {
    if cfg!(feature = "app-store") {
        file_bookmark(path).map(|bytes| to_hex(&bytes))
    } else {
        None
    }
}
