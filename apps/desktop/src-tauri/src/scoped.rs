//! Files found again from their bookmarks. In the Mac App Store build the app runs in App
//! Sandbox (ADR 0014 §11): a file the user opened earlier is reachable again only through its
//! security-scoped bookmark, and only while the access that resolving it starts lasts. This keeps
//! that access, one per bookmark, for as long as the app runs: the files are at most the recent list
//! and the session's tabs. Elsewhere bookmarks are plain and there is no access to keep.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use laminapdf_desktop_os::{FileAccess, file_bookmark, resolve_bookmark};

use crate::session::{from_hex, to_hex};

/// What each bookmark found, by the bookmark (as hex): where the file is, and the access that
/// keeps it reachable. A bookmark is resolved again only once its file isn't there any more.
static FOUND: Mutex<BTreeMap<String, (PathBuf, FileAccess)>> = Mutex::new(BTreeMap::new());

/// Where the file `bookmark` names is now, kept reachable; `None` when it can't be found.
pub(crate) fn find(bookmark: &[u8]) -> Option<PathBuf> {
    let key = to_hex(bookmark);
    let mut found = FOUND.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((path, _)) = found.get(&key).filter(|(path, _)| path.is_file()) {
        return Some(path.clone());
    }
    let (path, access) = resolve_bookmark(bookmark)?;
    found.insert(key, (path.clone(), access));
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
