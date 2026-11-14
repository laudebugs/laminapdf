//! Bookmarks of files: on macOS an `NSURL` bookmark finds a file again after it was moved or
//! renamed on its volume, where a path would no longer lead to it. The app isn't sandboxed, so
//! these are plain bookmarks, without a security scope. Other OSes have none: [`file_bookmark`]
//! makes nothing and [`resolve_bookmark`] finds nothing.

use std::path::{Path, PathBuf};

/// A bookmark of the file at `path`; `None` when the OS can't make one (or has no bookmarks).
#[must_use]
pub fn file_bookmark(path: &Path) -> Option<Vec<u8>> {
    os::file_bookmark(path)
}

/// Where the file `bookmark` was made for is now; `None` when it can't be found. Never asks the
/// user and never mounts a volume.
#[must_use]
pub fn resolve_bookmark(bookmark: &[u8]) -> Option<PathBuf> {
    os::resolve_bookmark(bookmark)
}

#[cfg(target_os = "macos")]
mod os {
    #![expect(unsafe_code, reason = "Foundation's bookmark resolution writes through a pointer")]

    use std::path::{Path, PathBuf};

    use objc2::runtime::Bool;
    use objc2_foundation::{NSData, NSString, NSURL, NSURLBookmarkCreationOptions, NSURLBookmarkResolutionOptions};

    pub(super) fn file_bookmark(path: &Path) -> Option<Vec<u8>> {
        // A path that isn't UTF-8 can't be an NSString without loss: it would name another file.
        let url = NSURL::fileURLWithPath(&NSString::from_str(path.to_str()?));
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                NSURLBookmarkCreationOptions::empty(),
                None,
                None,
            )
            .ok()?;
        Some(data.to_vec())
    }

    pub(super) fn resolve_bookmark(bookmark: &[u8]) -> Option<PathBuf> {
        let data = NSData::with_bytes(bookmark);
        let mut stale = Bool::NO;
        let options = NSURLBookmarkResolutionOptions::WithoutUI | NSURLBookmarkResolutionOptions::WithoutMounting;
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
        Some(PathBuf::from(url.path()?.to_string()))
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    use std::path::{Path, PathBuf};

    pub(super) const fn file_bookmark(_path: &Path) -> Option<Vec<u8>> {
        None
    }

    pub(super) const fn resolve_bookmark(_bookmark: &[u8]) -> Option<PathBuf> {
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
        let found = resolve_bookmark(&bookmark);
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
        let found = resolve_bookmark(&bookmark);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(found, None);
    }

    #[test]
    fn rubbish_should_resolve_to_nothing() {
        assert_eq!(resolve_bookmark(b"not a bookmark"), None);
    }
}
