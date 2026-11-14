//! The last session (D4d): the windows, their tabs and each tab's view, kept in `session.json`
//! in the app's config folder so a relaunch brings them back.
//!
//! Each tab keeps its own reference to its file, apart from the recent list (which the user can
//! clear or turn off and which holds 20 files): the canonical path, the size and modification
//! time it had, and on macOS a bookmark, which finds a file moved or renamed between sessions.
//! The file names documents as the recent list does, so it follows the same switch: it is written
//! only while "Remember recent files" and "Reopen last session" are both on, and deleted when
//! either turns off ([`should_persist`]).

pub(crate) mod app;
pub(crate) mod restore;
pub(crate) mod track;

use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::store::write_json;

/// The session file's name in the app's config folder.
pub(crate) const FILE_NAME: &str = "session.json";

/// The version of the file's layout; a file of another version is left unread.
pub(crate) const VERSION: u32 = 1;

/// What `session.json` holds: the windows, front to back by their labels' order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SessionFile {
    pub v: u32,
    pub windows: Vec<WindowRecord>,
}

/// One window: its label, its tabs in order and the shown one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WindowRecord {
    pub label: String,
    pub tabs: Vec<TabRef>,
    /// The shown tab's index in `tabs`.
    pub active: Option<usize>,
}

/// A tab's document and its view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TabRef {
    /// Canonical and absolute.
    pub path: PathBuf,
    pub stamp: Stamp,
    /// macOS: an `NSURL` bookmark of the file, hex-encoded. Found before the path when it resolves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
    /// The webview's view of the tab (page, offset, zoom, rotation); Rust passes it back unread.
    pub view: serde_json::Value,
}

/// The size and modification time the file had when the session was saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stamp {
    pub len: u64,
    /// Milliseconds since the Unix epoch; `None` where the OS gives no time.
    pub mtime: Option<u64>,
}

impl Stamp {
    /// `path`'s stamp now.
    pub(crate) fn of(path: &Path) -> io::Result<Self> {
        let meta = std::fs::metadata(path)?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|since| u64::try_from(since.as_millis()).ok());
        Ok(Self { len: meta.len(), mtime })
    }
}

/// Why the session file couldn't be used. The caller starts with no session.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionError {
    #[error("the session file could not be read: {0}")]
    Io(#[from] io::Error),
    #[error("the session file is damaged: {0}")]
    Damaged(#[from] serde_json::Error),
    #[error("the session file is version {0}, which this version of the app doesn't read")]
    Version(u32),
}

/// Whether the session is kept: only while both switches are on.
pub(crate) const fn should_persist(remember_recent: bool, reopen_last: bool) -> bool {
    remember_recent && reopen_last
}

/// The session saved in `dir`; `None` when there is none.
///
/// # Errors
/// [`SessionError`] for a file that can't be read, isn't the session's JSON, or is another version.
pub(crate) fn load(dir: &Path) -> Result<Option<SessionFile>, SessionError> {
    // The version first, so a newer layout is refused for its version, not as damage.
    #[derive(Deserialize)]
    struct Version {
        v: u32,
    }
    let bytes = match std::fs::read(dir.join(FILE_NAME)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let Version { v } = serde_json::from_slice(&bytes)?;
    if v != VERSION {
        return Err(SessionError::Version(v));
    }
    Ok(Some(serde_json::from_slice(&bytes)?))
}

/// Writes `session` to `dir`, atomically.
///
/// # Errors
/// The OS's error when the folder or the file can't be written.
pub(crate) fn save(dir: &Path, session: &SessionFile) -> io::Result<()> {
    write_json(&dir.join(FILE_NAME), session)
}

/// Deletes the session file in `dir`, if there is one.
///
/// # Errors
/// The OS's error, but not for a file that is missing already.
pub(crate) fn delete(dir: &Path) -> io::Result<()> {
    match std::fs::remove_file(dir.join(FILE_NAME)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Where a tab's file is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Resolved {
    /// At its path.
    Found(PathBuf),
    /// Somewhere else, found through its bookmark.
    Moved(PathBuf),
    /// Neither the bookmark nor the path finds it.
    Missing,
}

/// Finds a bookmark's file: the OS's resolver on macOS, none elsewhere.
pub(crate) trait Bookmarks {
    /// The path `bookmark` points at now, if it still finds a file.
    fn resolve(&self, bookmark: &[u8]) -> Option<PathBuf>;
}

/// `tab`'s file: through its bookmark first, then its path, else missing.
pub(crate) fn resolve(tab: &TabRef, bookmarks: &dyn Bookmarks) -> Resolved {
    let found = tab
        .bookmark
        .as_deref()
        .and_then(from_hex)
        .and_then(|bytes| bookmarks.resolve(&bytes))
        .filter(|path| path.is_file());
    match found {
        Some(path) if path == tab.path => Resolved::Found(path),
        Some(path) => Resolved::Moved(path),
        None if tab.path.is_file() => Resolved::Found(tab.path.clone()),
        None => Resolved::Missing,
    }
}

/// `bytes` as lowercase hex.
pub(crate) fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// The bytes of `hex`; `None` for an odd length or a character that isn't a hex digit.
pub(crate) fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    hex.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-session-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn tab(path: &Path, bookmark: Option<&[u8]>) -> TabRef {
        TabRef {
            path: path.to_owned(),
            stamp: Stamp { len: 1, mtime: Some(2) },
            bookmark: bookmark.map(to_hex),
            view: serde_json::json!({ "page": 3, "zoom": 1.5 }),
        }
    }

    fn session(path: &Path) -> SessionFile {
        SessionFile {
            v: VERSION,
            windows: vec![WindowRecord {
                label: "main".into(),
                tabs: vec![tab(path, Some(b"bm"))],
                active: Some(0),
            }],
        }
    }

    /// A resolver that knows a fixed set of bookmarks.
    struct Fake(BTreeMap<Vec<u8>, PathBuf>);

    impl Bookmarks for Fake {
        fn resolve(&self, bookmark: &[u8]) -> Option<PathBuf> {
            self.0.get(bookmark).cloned()
        }
    }

    fn none() -> Fake {
        Fake(BTreeMap::new())
    }

    #[test]
    fn a_saved_session_should_load_again() {
        let dir = temp_dir("roundtrip");
        let saved = session(&dir.join("a.pdf"));
        save(&dir, &saved).unwrap();
        let loaded = load(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded, Some(saved));
    }

    #[test]
    fn no_session_file_should_load_as_none() {
        let dir = temp_dir("none");
        let loaded = load(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(loaded, None);
    }

    #[test]
    fn a_damaged_session_file_should_fail_as_damaged() {
        let dir = temp_dir("damaged");
        std::fs::write(dir.join(FILE_NAME), b"{ not json").unwrap();
        let loaded = load(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(loaded, Err(SessionError::Damaged(_))));
    }

    #[test]
    fn another_version_should_fail_as_that_version() {
        let dir = temp_dir("version");
        std::fs::write(dir.join(FILE_NAME), br#"{ "v": 2, "windows": "a later layout" }"#).unwrap();
        let loaded = load(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(loaded, Err(SessionError::Version(2))));
    }

    #[test]
    fn deleting_should_remove_the_file_and_accept_none() {
        let dir = temp_dir("delete");
        save(&dir, &session(&dir.join("a.pdf"))).unwrap();
        delete(&dir).unwrap();
        let again = delete(&dir);
        let left = dir.join(FILE_NAME).exists();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(again.is_ok() && !left);
    }

    #[test]
    fn the_session_should_persist_only_with_both_switches_on() {
        let table = [(true, true), (true, false), (false, true), (false, false)].map(|(r, o)| should_persist(r, o));
        assert_eq!(table, [true, false, false, false]);
    }

    #[test]
    fn a_bookmark_that_finds_the_file_elsewhere_should_win_over_the_path() {
        let dir = temp_dir("moved");
        let (old, new) = (dir.join("old.pdf"), dir.join("new.pdf"));
        std::fs::write(&old, b"x").unwrap();
        std::fs::write(&new, b"x").unwrap();
        let bookmarks = Fake(BTreeMap::from([(b"bm".to_vec(), new.clone())]));
        let resolved = resolve(&tab(&old, Some(b"bm")), &bookmarks);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Resolved::Moved(new));
    }

    #[test]
    fn a_bookmark_that_finds_the_same_path_should_be_found() {
        let dir = temp_dir("same");
        let path = dir.join("a.pdf");
        std::fs::write(&path, b"x").unwrap();
        let bookmarks = Fake(BTreeMap::from([(b"bm".to_vec(), path.clone())]));
        let resolved = resolve(&tab(&path, Some(b"bm")), &bookmarks);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Resolved::Found(path));
    }

    #[test]
    fn the_path_should_serve_when_the_bookmark_finds_nothing() {
        let dir = temp_dir("path");
        let path = dir.join("a.pdf");
        std::fs::write(&path, b"x").unwrap();
        let resolved = resolve(&tab(&path, Some(b"bm")), &none());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Resolved::Found(path));
    }

    #[test]
    fn a_file_neither_finds_should_be_missing() {
        let dir = temp_dir("missing");
        let resolved = resolve(&tab(&dir.join("gone.pdf"), Some(b"bm")), &none());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Resolved::Missing);
    }

    #[test]
    fn a_bookmark_pointing_at_nothing_should_fall_back_to_the_path() {
        let dir = temp_dir("stale");
        let path = dir.join("a.pdf");
        std::fs::write(&path, b"x").unwrap();
        let bookmarks = Fake(BTreeMap::from([(b"bm".to_vec(), dir.join("deleted.pdf"))]));
        let resolved = resolve(&tab(&path, Some(b"bm")), &bookmarks);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Resolved::Found(path));
    }

    #[test]
    fn hex_should_round_trip_and_refuse_bad_input() {
        let bytes = [0_u8, 15, 16, 255];
        assert_eq!(
            (from_hex(&to_hex(&bytes)), from_hex("abc"), from_hex("zz")),
            (Some(bytes.to_vec()), None, None)
        );
    }
}
