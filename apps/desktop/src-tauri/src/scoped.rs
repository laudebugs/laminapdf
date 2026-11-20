//! Files found again from their bookmarks. In the Mac App Store build the app runs in App
//! Sandbox (ADR 0014 §11): a file the user opened earlier is reachable again only through its
//! security-scoped bookmark, and only while the access that resolving it starts lasts. This keeps
//! that access, one per bookmark, for as long as the app runs: the files are at most the recent list
//! and the session's tabs. Elsewhere bookmarks are plain and there is no access to keep.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use laminapdf_desktop_os::{FileAccess, FileIdentity, file_bookmark, file_identity, resolve_bookmark};

use crate::session::{from_hex, to_hex};

/// What each bookmark found, by the bookmark (as hex): where the file is, which file it was, and
/// the access that keeps it reachable. A bookmark is resolved again once that path holds another
/// file or none.
static FOUND: Mutex<BTreeMap<String, Found>> = Mutex::new(BTreeMap::new());

/// One bookmark's file.
struct Found {
    path: PathBuf,
    identity: Option<FileIdentity>,
    _access: FileAccess,
}

/// The identity of the file at `path`, if it can be read.
fn identity(path: &Path) -> Option<FileIdentity> {
    std::fs::File::open(path).and_then(|file| file_identity(&file)).ok()
}

/// Where the file `bookmark` names is now, kept reachable; `None` when it can't be found.
pub(crate) fn find(bookmark: &[u8]) -> Option<PathBuf> {
    let key = to_hex(bookmark);
    let mut found = FOUND.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(known) = found.get(&key)
        && known.identity.is_some()
        && identity(&known.path) == known.identity
    {
        return Some(known.path.clone());
    }
    // A bookmark of a file that was deleted or left its volume finds nothing: the caller drops it.
    let (path, access) = resolve_bookmark(bookmark).ok()?;
    found.insert(
        key,
        Found {
            identity: identity(&path),
            path: path.clone(),
            _access: access,
        },
    );
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
        bookmark(path)
    } else {
        None
    }
}

/// A bookmark of `path`, as hex; `None` on an OS without bookmarks, and with a warning saying
/// why when macOS refuses one. The file is then reached by its path, as in the direct download.
pub(crate) fn bookmark(path: &Path) -> Option<String> {
    match file_bookmark(path) {
        Ok(bytes) => Some(to_hex(&bytes)),
        Err(error) => {
            if error.kind() != std::io::ErrorKind::Unsupported {
                tracing::warn!("could not bookmark a file: {error}");
            }
            None
        }
    }
}
