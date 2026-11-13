//! How the main process opens a document's file ([`open_shared`]), and which file an open handle
//! reads ([`FileIdentity`]). The identity is asked of the handle itself: asking by path would open
//! the file a second time, and a rename between the two opens would name another file.

use std::fs::File;
use std::io;
use std::path::Path;

/// `path` opened read-only. On Windows it shares reading, writing and deleting, so other apps may
/// write, rename and delete the file while it is open.
///
/// # Errors
/// The OS's error when the file can't be opened.
pub fn open_shared(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;

        use windows::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};
        options.share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0);
    }
    options.open(path)
}

/// The file an open handle reads: its volume and file ID on Windows, its device and inode on
/// Unix. Another file at the same path, such as one an atomic save renamed over it, has another
/// identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileIdentity {
    volume: u64,
    file: u128,
}

/// The identity of the file `file` reads.
///
/// # Errors
/// The OS's error when it can't say, and `Unsupported` on an OS that has no such identity.
pub fn file_identity(file: &File) -> io::Result<FileIdentity> {
    os::identity(file)
}

#[cfg(unix)]
mod os {
    use std::fs::File;
    use std::io;
    use std::os::unix::fs::MetadataExt as _;

    use super::FileIdentity;

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let meta = file.metadata()?;
        Ok(FileIdentity {
            volume: meta.dev(),
            file: u128::from(meta.ino()),
        })
    }
}

#[cfg(windows)]
mod os {
    #![expect(unsafe_code, reason = "Windows' file information calls")]

    use std::fs::File;
    use std::io;
    use std::os::windows::io::AsRawHandle as _;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO, FileIdInfo, GetFileInformationByHandle, GetFileInformationByHandleEx,
    };

    use super::FileIdentity;

    /// The 128-bit file ID, which some file systems need whole, else the 64-bit file index on
    /// file systems that don't answer `FileIdInfo`. One file system always answers the same way, so two
    /// identities of the same path compare like with like.
    /// `FILE_ID_INFO`'s size, which fits a `u32` (checked when compiling).
    const ID_INFO_SIZE: u32 = {
        let size = size_of::<FILE_ID_INFO>();
        assert!(size <= u32::MAX as usize);
        #[expect(clippy::cast_possible_truncation, reason = "the assertion above bounds it")]
        let size = size as u32;
        size
    };

    pub(super) fn identity(file: &File) -> io::Result<FileIdentity> {
        let handle = HANDLE(file.as_raw_handle());
        let mut info = FILE_ID_INFO::default();
        // SAFETY: `handle` is open for as long as `file` is borrowed, and `info` is a writable
        // FILE_ID_INFO of exactly ID_INFO_SIZE bytes, the buffer FileIdInfo fills.
        let by_id = unsafe { GetFileInformationByHandleEx(handle, FileIdInfo, (&raw mut info).cast(), ID_INFO_SIZE) };
        if by_id.is_ok() {
            return Ok(FileIdentity {
                volume: info.VolumeSerialNumber,
                file: u128::from_le_bytes(info.FileId.Identifier),
            });
        }
        let mut old = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: `handle` is open for as long as `file` is borrowed, and `old` is a writable
        // BY_HANDLE_FILE_INFORMATION.
        unsafe { GetFileInformationByHandle(handle, &raw mut old) }.map_err(io::Error::from)?;
        Ok(FileIdentity {
            volume: u64::from(old.dwVolumeSerialNumber),
            file: (u128::from(old.nFileIndexHigh) << 32) | u128::from(old.nFileIndexLow),
        })
    }
}

#[cfg(not(any(unix, windows)))]
mod os {
    use std::fs::File;
    use std::io;

    use super::FileIdentity;

    pub(super) fn identity(_file: &File) -> io::Result<FileIdentity> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "no file identity on this OS",
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;
    use std::path::PathBuf;

    use super::*;

    fn temp(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("laminapdf-desktop-os-{}-{name}", std::process::id()));
        File::create(&path).unwrap().write_all(bytes).unwrap();
        path
    }

    #[test]
    fn two_handles_on_one_file_should_have_one_identity() {
        let path = temp("same", b"one");
        let (a, b) = (File::open(&path).unwrap(), File::open(&path).unwrap());
        let same = file_identity(&a).unwrap() == file_identity(&b).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(same);
    }

    #[test]
    fn a_file_renamed_over_another_should_have_a_new_identity() {
        let path = temp("replaced", b"the first version");
        let first = file_identity(&File::open(&path).unwrap()).unwrap();
        let saved = temp("replacement", b"the second version");
        std::fs::rename(&saved, &path).unwrap();
        let second = file_identity(&File::open(&path).unwrap()).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_ne!(first, second);
    }
}
