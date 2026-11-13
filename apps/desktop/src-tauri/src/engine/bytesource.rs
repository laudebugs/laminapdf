//! [`FileSource`]: one open document's file, read by position for the engine.
//!
//! No memory mapping (ADR 0008 §2): if another app truncates a mapped file, touching the mapping
//! raises `SIGBUS` (`EXCEPTION_IN_PAGE_ERROR` on Windows) in the main process and takes down every
//! window. A positioned read of a truncated file is an ordinary error instead.
//!
//! On Windows no handle stays open between reads (ADR 0008 §11): `MoveFileEx` and `ReplaceFile`,
//! which some apps' atomic saves use, refuse to rename over a file any process holds open, whatever
//! the share mode. Each read opens the path again and checks that it is still the file that
//! opened: a file replaced since fails the read, so the old document never gets the new file's
//! bytes, and the watcher reloads the new one. Elsewhere a rename already replaces an open file,
//! and the one handle keeps reading the file that opened.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use laminapdf_desktop_os::{FileIdentity, file_identity};

/// The most bytes one read returns. The engine asks for 256 KiB blocks; a larger request comes
/// from a confused or compromised engine and must not make the main process allocate gigabytes.
pub(crate) const MAX_READ: u32 = 16 * 1024 * 1024;

/// A document's file, opened read-only. Its length is fixed when it opens: the engine
/// reads that many bytes, and a file that shrinks underneath answers errors.
#[derive(Debug)]
pub(crate) struct FileSource {
    access: Access,
    len: u64,
}

/// How reads reach the file.
#[derive(Debug)]
enum Access {
    /// One handle, opened once: macOS and Linux.
    Held(File),
    /// The path, opened for each read and checked against the file that opened: Windows.
    Reopened { path: PathBuf, identity: FileIdentity },
}

impl FileSource {
    /// Opens `path` read-only. Other apps may still write, rename, delete and replace it.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        Self::open_with(path, cfg!(windows))
    }

    /// Opens `path`, holding its handle, or (`reopen`) keeping only its path and identity.
    fn open_with(path: &Path, reopen: bool) -> io::Result<Self> {
        let file = open_shared(path)?;
        let len = file.metadata()?.len();
        let access = if reopen {
            Access::Reopened {
                path: path.to_owned(),
                identity: file_identity(&file)?,
            }
        } else {
            Access::Held(file)
        };
        Ok(Self { access, len })
    }

    /// The file's length when it opened.
    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    /// `len` bytes from `offset`, short only at the length the file had when it opened.
    ///
    /// # Errors
    /// `InvalidInput` for a read over [`MAX_READ`], `UnexpectedEof` for a read that starts past
    /// the end or a file that has shrunk, `Other` for a file replaced or grown since it opened, and the OS's
    /// own errors (`NotFound` for a file moved or deleted, on Windows).
    pub(crate) fn read(&self, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        if len > MAX_READ {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "read too large"));
        }
        if offset > self.len || (offset == self.len && len > 0) {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "read past the end of the file",
            ));
        }
        match &self.access {
            Access::Held(file) => self.read_from(file, offset, len),
            Access::Reopened { path, identity } => {
                let file = open_shared(path)?;
                // A file renamed over it has another identity, and one written in place to a
                // greater length is no longer the file that opened. A shorter one fails as a held
                // handle's read does.
                let now = file.metadata()?.len();
                if file_identity(&file)? != *identity || now > self.len {
                    return Err(io::Error::other("the file was replaced since it opened"));
                }
                if now < self.len {
                    return Err(shorter());
                }
                self.read_from(&file, offset, len)
            }
        }
    }

    fn read_from(&self, file: &File, offset: u64, len: u32) -> io::Result<Vec<u8>> {
        let want = u64::from(len).min(self.len - offset);
        let mut buf = vec![0_u8; usize::try_from(want).unwrap_or(0)];
        let mut filled = 0;
        while let Some(rest) = buf.get_mut(filled..).filter(|rest| !rest.is_empty()) {
            match read_at(file, rest, offset + filled as u64) {
                Ok(0) => return Err(shorter()),
                Ok(n) => filled += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(buf)
    }
}

fn shorter() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the file is shorter than when it was opened",
    )
}

/// `path` opened read-only. On Windows other apps may write, rename and delete it while it is open.
fn open_shared(path: &Path) -> io::Result<File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE.
        options.share_mode(0x1 | 0x2 | 0x4);
    }
    options.open(path)
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    /// A temporary file holding `bytes`, removed when dropped.
    struct Temp(PathBuf);

    impl Temp {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!("laminapdf-desktop-{}-{name}", std::process::id()));
            let mut file = File::create(&path).unwrap();
            file.write_all(bytes).unwrap();
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn source(name: &str, bytes: &[u8]) -> (Temp, FileSource) {
        let temp = Temp::new(name, bytes);
        let source = FileSource::open(&temp.0).unwrap();
        (temp, source)
    }

    #[test]
    fn a_read_inside_the_file_should_return_those_bytes() {
        let (_t, src) = source("inside", b"0123456789");
        assert_eq!(src.read(2, 4).unwrap(), b"2345");
    }

    #[test]
    fn a_read_over_the_end_should_stop_at_the_end() {
        let (_t, src) = source("over", b"0123456789");
        assert_eq!(src.read(8, 100).unwrap(), b"89");
    }

    #[test]
    fn a_zero_length_read_at_the_end_should_be_empty() {
        let (_t, src) = source("zero", b"0123456789");
        assert_eq!(src.read(10, 0).unwrap(), b"");
    }

    #[test]
    fn a_read_starting_past_the_end_should_fail() {
        let (_t, src) = source("past", b"0123456789");
        assert_eq!(src.read(11, 1).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_read_starting_at_the_end_should_fail() {
        let (_t, src) = source("end", b"0123456789");
        assert_eq!(src.read(10, 1).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_read_larger_than_the_limit_should_fail_without_allocating() {
        let (_t, src) = source("large", b"0123456789");
        assert_eq!(
            src.read(0, MAX_READ + 1).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn a_file_truncated_after_opening_should_fail_cleanly() {
        let (temp, src) = source("truncated", &[7; 4096]);
        File::options().write(true).open(&temp.0).unwrap().set_len(100).unwrap();
        assert_eq!(src.read(0, 4096).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    /// Another app saving over an open document (an atomic save renames its file over ours), then
    /// deleting it: neither is refused, and a held handle keeps reading what it opened.
    #[test]
    fn another_app_should_replace_and_delete_an_open_file() {
        let temp = Temp::new("replaced", b"the first version");
        let src = FileSource::open_with(&temp.0, false).unwrap();
        let saved = Temp::new("replaced-new", b"the second version, longer");
        std::fs::rename(&saved.0, &temp.0).unwrap();
        let after_replace = src.read(0, 9).unwrap();
        std::fs::remove_file(&temp.0).unwrap();
        assert_eq!(
            (after_replace, src.read(4, 5).unwrap()),
            (b"the first".to_vec(), b"first".to_vec())
        );
    }

    /// The Windows way, on every OS: a read after another app renamed a new file over the open one
    /// fails, and never returns the new file's bytes; a read before it returns the first file's.
    #[test]
    fn a_reopening_source_should_refuse_a_file_replaced_since_it_opened() {
        let temp = Temp::new("reopened", b"the first version");
        let src = FileSource::open_with(&temp.0, true).unwrap();
        let before = src.read(0, 9).unwrap();
        let saved = Temp::new("reopened-new", b"the second version");
        std::fs::rename(&saved.0, &temp.0).unwrap();
        let after = src.read(0, 9).unwrap_err();
        assert_eq!(
            (before, after.kind(), after.to_string()),
            (
                b"the first".to_vec(),
                io::ErrorKind::Other,
                "the file was replaced since it opened".to_owned()
            )
        );
    }

    #[test]
    fn a_reopening_source_should_refuse_a_file_written_longer() {
        let temp = Temp::new("regrown", b"0123456789");
        let src = FileSource::open_with(&temp.0, true).unwrap();
        std::fs::write(&temp.0, b"0123456789 and more").unwrap();
        assert_eq!(src.read(0, 4).unwrap_err().kind(), io::ErrorKind::Other);
    }

    #[test]
    fn a_reopening_source_should_fail_cleanly_on_a_file_truncated_after_opening() {
        let temp = Temp::new("retruncated", &[7; 4096]);
        let src = FileSource::open_with(&temp.0, true).unwrap();
        File::options().write(true).open(&temp.0).unwrap().set_len(100).unwrap();
        assert_eq!(src.read(0, 4096).unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn a_reopening_source_should_fail_once_its_file_is_deleted() {
        let temp = Temp::new("deleted", b"0123456789");
        let src = FileSource::open_with(&temp.0, true).unwrap();
        std::fs::remove_file(&temp.0).unwrap();
        assert_eq!(src.read(0, 4).unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_reopening_source_should_read_its_file_like_a_held_one() {
        let (temp, held) = source("both", b"0123456789");
        let reopened = FileSource::open_with(&temp.0, true).unwrap();
        assert_eq!(reopened.read(2, 100).unwrap(), held.read(2, 100).unwrap());
    }

    #[test]
    fn the_length_should_be_the_files_when_it_opened() {
        let (_t, src) = source("len", &[1; 1234]);
        assert_eq!(src.len(), 1234);
    }
}
