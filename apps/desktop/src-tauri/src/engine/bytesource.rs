//! [`FileSource`]: one open document's file, read by position for the engine.
//!
//! No memory mapping (ADR 0008 §2): if another app truncates a mapped file, touching the mapping
//! raises `SIGBUS` (`EXCEPTION_IN_PAGE_ERROR` on Windows) in the main process and takes down every
//! window. A positioned read of a truncated file is an ordinary error instead.

use std::fs::File;
use std::io;
use std::path::Path;

/// The most bytes one read returns. The engine asks for 256 KiB blocks; a larger request comes
/// from a confused or compromised engine and must not make the main process allocate gigabytes.
pub(crate) const MAX_READ: u32 = 16 * 1024 * 1024;

/// A document's file, opened read-only once. Its length is fixed when it opens: the engine
/// reads that many bytes, and a file that shrinks underneath answers errors.
#[derive(Debug)]
pub(crate) struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    /// Opens `path` read-only. On Windows other apps may still write, rename and delete it.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt as _;
            // FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE.
            options.share_mode(0x1 | 0x2 | 0x4);
        }
        let file = options.open(path)?;
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }

    /// The file's length when it opened.
    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    /// `len` bytes from `offset`, short only at the length the file had when it opened.
    ///
    /// # Errors
    /// `InvalidInput` for a read over [`MAX_READ`], `UnexpectedEof` for a read that starts past
    /// the end or a file that has shrunk, and the OS's own read errors.
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
        let want = u64::from(len).min(self.len - offset);
        let mut buf = vec![0_u8; usize::try_from(want).unwrap_or(0)];
        let mut filled = 0;
        while let Some(rest) = buf.get_mut(filled..).filter(|rest| !rest.is_empty()) {
            match read_at(&self.file, rest, offset + filled as u64) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the file is shorter than when it was opened",
                    ));
                }
                Ok(n) => filled += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(buf)
    }
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
    struct Temp(std::path::PathBuf);

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
    /// deleting it: neither is refused, and the open handle keeps reading what it opened. On
    /// Windows that needs the share mode `FileSource` opens with.
    #[test]
    fn another_app_should_replace_and_delete_an_open_file() {
        let (temp, src) = source("replaced", b"the first version");
        let saved = Temp::new("replaced-new", b"the second version, longer");
        std::fs::rename(&saved.0, &temp.0).unwrap();
        let after_replace = src.read(0, 9).unwrap();
        std::fs::remove_file(&temp.0).unwrap();
        assert_eq!(
            (after_replace, src.read(4, 5).unwrap()),
            (b"the first".to_vec(), b"first".to_vec())
        );
    }

    #[test]
    fn the_length_should_be_the_files_when_it_opened() {
        let (_t, src) = source("len", &[1; 1234]);
        assert_eq!(src.len(), 1234);
    }
}
