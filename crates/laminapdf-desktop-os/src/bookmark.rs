//! Bookmarks of files: on macOS an `NSURL` bookmark finds a file again after it was moved or
//! renamed on its volume, where a path would no longer lead to it. The direct download isn't
//! sandboxed, so its bookmarks are plain. The Mac App Store build (feature `security-scoped`,
//! ADR 0014 §11) runs in App Sandbox: its bookmarks carry a security scope, and resolving one
//! starts access to the file until the [`FileAccess`] it returns is dropped. Other OSes have
//! none: [`file_bookmark`] makes nothing and [`resolve_bookmark`] finds nothing.

use std::path::{Path, PathBuf};

/// A bookmark of the file at `path`; `None` when the OS can't make one (or has no bookmarks).
#[must_use]
pub fn file_bookmark(path: &Path) -> Option<Vec<u8>> {
    os::file_bookmark(path)
}

/// Where the file `bookmark` was made for is now, with access to it while the returned
/// [`FileAccess`] lives; `None` when it can't be found. Never asks the user and never mounts a
/// volume.
#[must_use]
pub fn resolve_bookmark(bookmark: &[u8]) -> Option<(PathBuf, FileAccess)> {
    os::resolve_bookmark(bookmark)
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

    use std::path::{Path, PathBuf};

    use objc2::rc::Retained;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSData, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions};

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

    /// The bookmark options: a security scope in the sandboxed store build.
    const CREATE: NSURLBookmarkCreationOptions = if cfg!(feature = "security-scoped") {
        NSURLBookmarkCreationOptions::WithSecurityScope
    } else {
        NSURLBookmarkCreationOptions::empty()
    };

    pub(super) fn file_bookmark(path: &Path) -> Option<Vec<u8>> {
        // A path that isn't UTF-8 can't be an NSString without loss: it would name another file.
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(CREATE, None, None)
            .ok()?;
        Some(data.to_vec())
    }

    pub(super) fn resolve_bookmark(bookmark: &[u8]) -> Option<(PathBuf, FileAccess)> {
        let data = NSData::with_bytes(bookmark);
        let mut stale = Bool::NO;
        let mut options = NSURLBookmarkResolutionOptions::WithoutUI | NSURLBookmarkResolutionOptions::WithoutMounting;
        if cfg!(feature = "security-scoped") {
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
        .ok()?;
        // A stale bookmark still names the file; the session saves a fresh one next time.
        let path = PathBuf::from(url.path()?.to_string());
        let access = if cfg!(feature = "security-scoped") {
            // SAFETY: `url` was resolved from a security-scoped bookmark just now; `Access`
            // stops this access exactly once, when it is dropped.
            let started = unsafe { url.startAccessingSecurityScopedResource() };
            Access(started.then_some(url))
        } else {
            Access(None)
        };
        Some((path, FileAccess { _access: access }))
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    use std::path::{Path, PathBuf};

    use super::FileAccess;

    /// No access to hold.
    #[derive(Debug, Default)]
    pub(super) struct Access;

    pub(super) const fn file_bookmark(_path: &Path) -> Option<Vec<u8>> {
        None
    }

    pub(super) const fn resolve_bookmark(_bookmark: &[u8]) -> Option<(PathBuf, FileAccess)> {
        None
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn a_bookmark_should_find_its_file_after_a_rename() {
        let dir = std::env::temp_dir().join(format!("laminapdf-bookmark-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dir = std::fs::canonicalize(&dir).unwrap();
        let (before, after) = (dir.join("before.pdf"), dir.join("after.pdf"));
        std::fs::write(&before, b"%PDF").unwrap();
        let bookmark = file_bookmark(&before).unwrap();
        std::fs::rename(&before, &after).unwrap();
        let found = resolve_bookmark(&bookmark).map(|(path, _)| path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, Some(after));
    }

    #[test]
    fn a_bookmark_of_a_deleted_file_should_find_nothing() {
        let dir = std::env::temp_dir().join(format!("laminapdf-bookmark-gone-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gone.pdf");
        std::fs::write(&path, b"%PDF").unwrap();
        let bookmark = file_bookmark(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let found = resolve_bookmark(&bookmark).map(|(path, _)| path);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, None);
    }

    #[test]
    fn rubbish_should_resolve_to_nothing() {
        assert!(resolve_bookmark(b"not a bookmark").is_none());
    }
}
