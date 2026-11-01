//! [`MmapSource`]: a memory-mapped file (opt in with the `mmap` feature).
#![expect(
    unsafe_code,
    reason = "memmap2's Mmap::map is unsafe: the mapping is only sound while no one changes the file"
)]

use std::fs::File;
use std::io;
use std::path::Path;

use laminapdf_cos::{ByteSource, SourceError};
use memmap2::Mmap;

/// A file mapped into memory: [`ByteSource::as_slice`] returns the mapping, so the parser
/// works on it in place with no copies.
///
/// **Only for files nothing else changes while the document is open.** If another process
/// truncates the file, touching a page past the new end raises `SIGBUS` (the process dies; it is
/// not an error lamina can catch), and changed bytes change under the parser. Prefer
/// [`FileSource`](crate::FileSource) unless the file is known to be immutable (a read-only
/// volume, a file the app wrote and keeps locked). An empty file maps as empty.
#[derive(Debug)]
pub struct MmapSource {
    map: Option<Mmap>,
}

impl MmapSource {
    /// Maps `path` read-only.
    ///
    /// # Errors
    /// The file can't be opened or mapped.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::from_file(&File::open(path)?)
    }

    /// Maps an open file read-only.
    ///
    /// # Errors
    /// The file can't be mapped.
    pub fn from_file(file: &File) -> io::Result<Self> {
        if file.metadata()?.len() == 0 {
            return Ok(Self { map: None });
        }
        // SAFETY: `Mmap::map` is unsafe because the mapping aliases the file: if the file is
        // modified or truncated while mapped, reads see changed bytes or fault with SIGBUS. The
        // type's documentation makes "nothing changes the file while it is open" the caller's
        // contract, as memmap2 requires; the mapping is read-only, so lamina never writes to it.
        let map = unsafe { Mmap::map(file)? };
        Ok(Self { map: Some(map) })
    }
}

impl ByteSource for MmapSource {
    fn len(&self) -> u64 {
        self.as_bytes().len() as u64
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        let bytes = self.as_bytes();
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let Some(rest) = bytes.get(start..) else {
            return Ok(0);
        };
        let n = rest.len().min(buf.len());
        if let (Some(dst), Some(src)) = (buf.get_mut(..n), rest.get(..n)) {
            dst.copy_from_slice(src);
        }
        Ok(n)
    }

    fn as_slice(&self) -> Option<&[u8]> {
        Some(self.as_bytes())
    }
}

impl MmapSource {
    fn as_bytes(&self) -> &[u8] {
        self.map.as_deref().unwrap_or_default()
    }
}
