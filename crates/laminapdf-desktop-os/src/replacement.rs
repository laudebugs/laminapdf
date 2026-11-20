//! A folder for writing a file that will replace another. In App Sandbox (the Mac App Store
//! build, ADR 0014 §11) the app may write only the files the user chose, not new files beside
//! them, so a save can't put its temporary file in the document's folder. macOS's item
//! replacement directory is a fresh folder on the same volume that the app may write to, from
//! which a rename over the document works, as `NSDocument`'s safe saving does.

use std::io;
use std::path::{Path, PathBuf};

/// A new, empty folder on `path`'s volume to write `path`'s replacement in, which the caller
/// removes when done.
///
/// # Errors
/// Not macOS (`Unsupported`), or the OS couldn't make one.
pub fn replacement_folder(path: &Path) -> io::Result<PathBuf> {
    os::replacement_folder(path)
}

#[cfg(target_os = "macos")]
mod os {
    use std::io;
    use std::path::{Path, PathBuf};

    use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask, NSString, NSURL};

    pub(super) fn replacement_folder(path: &Path) -> io::Result<PathBuf> {
        let text = path
            .to_str()
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(text));
        let folder = NSFileManager::defaultManager()
            .URLForDirectory_inDomain_appropriateForURL_create_error(
                NSSearchPathDirectory::ItemReplacementDirectory,
                NSSearchPathDomainMask::UserDomainMask,
                Some(&url),
                true,
            )
            .map_err(|error| io::Error::other(error.localizedDescription().to_string()))?;
        folder
            .path()
            .map(|folder| PathBuf::from(folder.to_string()))
            .ok_or_else(|| io::Error::other("the replacement folder has no path"))
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    use std::io;
    use std::path::{Path, PathBuf};

    pub(super) fn replacement_folder(_path: &Path) -> io::Result<PathBuf> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn the_folder_should_be_on_the_file_s_volume_and_take_a_rename_over_it() {
        let dir = std::env::temp_dir().join(format!("laminapdf-replace-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("doc.pdf");
        std::fs::write(&target, b"old").unwrap();
        let folder = replacement_folder(&target).unwrap();
        let temp = folder.join("doc.pdf");
        std::fs::write(&temp, b"new").unwrap();
        std::fs::rename(&temp, &target).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        let read = std::fs::read(&target).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read, b"new");
    }
}
