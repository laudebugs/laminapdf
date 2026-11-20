//! Bookmarks of files: on macOS an `NSURL` bookmark finds a file again after it was moved or
//! renamed on its volume, where a path would no longer lead to it. The direct download isn't
//! sandboxed, so its bookmarks are plain. The Mac App Store build (feature `security-scoped`,
//! ADR 0014 §11) runs in App Sandbox: its bookmarks carry a security scope, and resolving one
//! starts access to the file until the [`FileAccess`] it returns is dropped. Other OSes have
//! none: [`file_bookmark`] and [`resolve_bookmark`] fail with `Unsupported`.

use std::io;
use std::path::{Path, PathBuf};

/// Whether bookmarks carry a security scope: only in the sandboxed store build.
const SCOPED: bool = cfg!(feature = "security-scoped");

/// A bookmark of the file at `path`.
///
/// # Errors
/// Not macOS (`Unsupported`), a path that isn't UTF-8 (`InvalidInput`), or the OS's refusal with
/// its error domain and code. A security-scoped bookmark needs a code-signed process: an unsigned
/// one is refused.
pub fn file_bookmark(path: &Path) -> io::Result<Vec<u8>> {
    os::file_bookmark(path, SCOPED)
}

/// Where the file `bookmark` was made for is now, with access to it while the returned
/// [`FileAccess`] lives. Never asks the user and never mounts a volume.
///
/// # Errors
/// Not macOS (`Unsupported`), or the OS's error when it can't find the file (moved off its
/// volume, deleted, or not a bookmark at all).
pub fn resolve_bookmark(bookmark: &[u8]) -> io::Result<(PathBuf, FileAccess)> {
    os::resolve_bookmark(bookmark, SCOPED)
}

/// Access to a file a bookmark found. In App Sandbox the file stays reachable until this is
/// dropped; elsewhere it holds nothing.
#[derive(Debug, Default)]
pub struct FileAccess {
    /// Held for its `Drop`, which ends the access.
    _access: os::Access,
}

#[cfg(target_os = "macos")]
mod os {
    #![expect(unsafe_code, reason = "Foundation's bookmark resolution writes through a pointer")]

    use std::io;
    use std::path::{Path, PathBuf};

    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2_foundation::{
        NSData, NSError, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions,
    };

    use super::FileAccess;

    /// A security-scoped URL whose access was started, stopped on drop.
    #[derive(Debug, Default)]
    pub(super) struct Access(Option<Retained<NSURL>>);

    // SAFETY: NSURL is immutable and Foundation documents it as safe to use from any thread;
    // the only call made through it, `stopAccessingSecurityScopedResource`, is thread-safe too.
    unsafe impl Send for Access {}
    // SAFETY: as above; `&Access` exposes nothing.
    unsafe impl Sync for Access {}

    impl Drop for Access {
        fn drop(&mut self) {
            if let Some(url) = self.0.take() {
                // SAFETY: `url` came from resolving a security-scoped bookmark, and its access
                // was started once in `resolve_bookmark`; this ends it once.
                unsafe { url.stopAccessingSecurityScopedResource() };
            }
        }
    }

    /// The OS's refusal, with its domain and code, which say why (for a security-scoped
    /// bookmark, often `NSCocoaErrorDomain` 256 from the agent that signs it).
    fn refused(error: &NSError) -> io::Error {
        io::Error::other(format!(
            "{} error {}: {}",
            error.domain(),
            error.code(),
            error.localizedDescription()
        ))
    }

    pub(super) fn file_bookmark(path: &Path, scoped: bool) -> io::Result<Vec<u8>> {
        // A path that isn't UTF-8 can't be an NSString without loss: it would name another file.
        let text = path
            .to_str()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(text));
        let options = if scoped {
            NSURLBookmarkCreationOptions::WithSecurityScope
        } else {
            NSURLBookmarkCreationOptions::empty()
        };
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(options, None, None)
            .map_err(|error| refused(&error))?;
        Ok(data.to_vec())
    }

    pub(super) fn resolve_bookmark(bookmark: &[u8], scoped: bool) -> io::Result<(PathBuf, FileAccess)> {
        let data = NSData::with_bytes(bookmark);
        let mut stale = Bool::NO;
        let mut options = NSURLBookmarkResolutionOptions::WithoutUI | NSURLBookmarkResolutionOptions::WithoutMounting;
        if scoped {
            options |= NSURLBookmarkResolutionOptions::WithSecurityScope;
        }
        // SAFETY: `stale` is a valid, writable Bool that outlives the call; the other arguments
        // are owned values or None.
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                options,
                None,
                &raw mut stale,
            )
        }
        .map_err(|error| refused(&error))?;
        // A stale bookmark still names the file; the session saves a fresh one next time.
        let path = url
            .path()
            .map(|path| PathBuf::from(path.to_string()))
            .ok_or_else(|| io::Error::other("the bookmark's URL has no path"))?;
        let access = if scoped {
            // SAFETY: `url` was resolved from a security-scoped bookmark just now; `Access`
            // stops this access exactly once, when it is dropped.
            let started = unsafe { url.startAccessingSecurityScopedResource() };
            Access(started.then_some(url))
        } else {
            Access(None)
        };
        Ok((path, FileAccess { _access: access }))
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    use std::io;
    use std::path::{Path, PathBuf};

    use super::FileAccess;

    /// No access to hold.
    #[derive(Debug, Default)]
    pub(super) struct Access;

    pub(super) fn file_bookmark(_path: &Path, _scoped: bool) -> io::Result<Vec<u8>> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }

    pub(super) fn resolve_bookmark(_bookmark: &[u8], _scoped: bool) -> io::Result<(PathBuf, FileAccess)> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

// Two attributes: clippy allows unwrap in the helpers only under a plain `cfg(test)`.
#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;

    /// A fresh folder in the temp dir, its symlinks resolved (`/var` is `/private/var`), so a
    /// path a bookmark resolves to compares equal.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("laminapdf-bookmark-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::canonicalize(&dir).unwrap()
    }

    /// Where a bookmark of a file finds it after a rename.
    fn found_after_rename(name: &str, scoped: bool) -> (PathBuf, PathBuf) {
        let dir = folder(name);
        let (before, after) = (dir.join("before.pdf"), dir.join("after.pdf"));
        std::fs::write(&before, b"%PDF").unwrap();
        let bookmark = os::file_bookmark(&before, scoped).unwrap();
        std::fs::rename(&before, &after).unwrap();
        let found = os::resolve_bookmark(&bookmark, scoped).map(|(path, _)| path);
        let _ = std::fs::remove_dir_all(&dir);
        (found.unwrap(), after)
    }

    /// Whether a bookmark of a deleted file still resolves.
    fn resolves_after_delete(name: &str, scoped: bool) -> bool {
        let dir = folder(name);
        let path = dir.join("gone.pdf");
        std::fs::write(&path, b"%PDF").unwrap();
        let bookmark = os::file_bookmark(&path, scoped).unwrap();
        std::fs::remove_file(&path).unwrap();
        let found = os::resolve_bookmark(&bookmark, scoped);
        let _ = std::fs::remove_dir_all(&dir);
        found.is_ok()
    }

    #[test]
    fn a_bookmark_should_find_its_file_after_a_rename() {
        let (found, after) = found_after_rename("plain", false);
        assert_eq!(found, after);
    }

    #[test]
    fn a_bookmark_of_a_deleted_file_should_find_nothing() {
        assert!(!resolves_after_delete("plain-gone", false));
    }

    /// Needs a code-signed test binary: the linker signs it ad hoc (this crate's build script).
    #[cfg(feature = "security-scoped")]
    #[test]
    fn a_security_scoped_bookmark_should_find_its_file_after_a_rename() {
        let (found, after) = found_after_rename("scoped", true);
        assert_eq!(found, after);
    }

    #[cfg(feature = "security-scoped")]
    #[test]
    fn a_security_scoped_bookmark_of_a_deleted_file_should_find_nothing() {
        assert!(!resolves_after_delete("scoped-gone", true));
    }

    #[test]
    fn rubbish_should_resolve_to_nothing() {
        assert!(resolve_bookmark(b"not a bookmark").is_err());
    }
}
