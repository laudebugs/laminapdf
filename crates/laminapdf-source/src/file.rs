//! [`FileSource`]: a file read with positioned reads (no mapping, no `unsafe`).

use std::fs::File;
use std::io;
use std::path::Path;

use laminapdf_cos::{ByteSource, SourceError};

/// A file read on demand with positioned reads (`pread` on Unix, `seek_read` on Windows), so
/// several threads can read it at once without a shared cursor. The length is taken at open;
/// a file that shrinks afterwards answers [`SourceError::Failed`] for the missing bytes, never
/// a crash. The default for large local files (D2b keeps mmap out of the desktop process).
#[derive(Debug)]
pub struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    /// Opens `path` for reading.
    ///
    /// # Errors
    /// The file can't be opened or its length read.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_file(File::open(path)?)
    }

    /// Wraps an open file; its current length is the source's length.
    ///
    /// # Errors
    /// The length can't be read.
    pub fn from_file(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        Ok(Self { file, len })
    }
}

#[cfg(unix)]
fn read_once(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_once(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}

impl ByteSource for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        if offset >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let want = usize::try_from(self.len - offset).map_or(buf.len(), |rest| rest.min(buf.len()));
        let failed = || SourceError::Failed {
            offset,
            len: want as u64,
        };
        let mut done = 0;
        while done < want {
            let (Some(dst), Some(at)) = (buf.get_mut(done..want), offset.checked_add(done as u64)) else {
                return Err(failed());
            };
            match read_once(&self.file, dst, at) {
                // End of file before the length taken at open: the file was truncated.
                Ok(0) => return Err(failed()),
                Ok(n) => done += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return Err(failed()),
            }
        }
        Ok(done)
    }
}
