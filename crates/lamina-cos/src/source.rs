//! The input a [`Document`](crate::Document) owns: [`ByteSource`] ([ADR 0005] §2).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

use std::fmt;
use std::sync::Arc;

/// Random-access input bytes: in memory, memory-mapped, a browser `Blob`, an HTTP range cache.
///
/// Synchronous by design (the core has no async runtime): a source whose bytes have not
/// arrived yet answers [`SourceError::Pending`], and the shell fetches them and retries.
/// In-memory sources return their bytes from [`ByteSource::as_slice`], which lets the
/// parser work on them in place.
///
/// Core implementations: `Vec<u8>` (moved in), `Arc<[u8]>` (shared), `&'static [u8]`.
/// Platform sources (mmap, `Blob`, OPFS) live in edge crates.
pub trait ByteSource: Send + Sync {
    /// Total length in bytes.
    fn len(&self) -> u64;

    /// Whether the source holds no bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Fills `buf` from `offset`; returns the bytes read (short only at the end of input).
    ///
    /// # Errors
    /// [`SourceError::Pending`] when the range is not available yet, [`SourceError::Failed`]
    /// when it cannot be read.
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError>;

    /// The whole input, when it is already in memory (lets the parser skip copies).
    fn as_slice(&self) -> Option<&[u8]> {
        None
    }
}

/// Why a [`ByteSource`] could not deliver a range.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// The range has not been fetched yet; fetch it and retry the call.
    #[error("bytes {offset}..+{len} are not available yet")]
    Pending {
        /// First byte of the missing range.
        offset: u64,
        /// Length of the missing range.
        len: u64,
    },
    /// The range cannot be read.
    #[error("bytes {offset}..+{len} could not be read")]
    Failed {
        /// First byte of the failed range.
        offset: u64,
        /// Length of the failed range.
        len: u64,
    },
}

/// Copies from an in-memory slice (the shared [`ByteSource::read_at`] of the core impls).
fn read_slice(bytes: &[u8], offset: u64, buf: &mut [u8]) -> usize {
    let Some(tail) = usize::try_from(offset).ok().and_then(|o| bytes.get(o..)) else {
        return 0;
    };
    let n = tail.len().min(buf.len());
    if let (Some(dst), Some(src)) = (buf.get_mut(..n), tail.get(..n)) {
        dst.copy_from_slice(src);
    }
    n
}

macro_rules! in_memory_source {
    ($ty:ty) => {
        impl ByteSource for $ty {
            fn len(&self) -> u64 {
                u64::try_from(<[u8]>::len(self)).unwrap_or(u64::MAX)
            }

            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
                Ok(read_slice(self, offset, buf))
            }

            fn as_slice(&self) -> Option<&[u8]> {
                let bytes: &[u8] = self;
                Some(bytes)
            }
        }
    };
}

in_memory_source!(Vec<u8>);
in_memory_source!(Arc<[u8]>);
in_memory_source!(&'static [u8]);

/// A shared [`ByteSource`]: what [`Document::open`](crate::Document::open) takes.
///
/// Every [`ByteSource`] converts into one (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`, or your
/// own type), and so does an `Arc<dyn ByteSource>`, which lets several documents (one per
/// thread, ADR 0005 §7) share one input without copying it.
///
/// ```
/// use std::sync::Arc;
/// use lamina_cos::{ByteSource, Source};
///
/// let shared: Arc<dyn ByteSource> = Arc::new(b"%PDF-1.7".to_vec());
/// let a = Source::from(Arc::clone(&shared));
/// let b = Source::from(shared);
/// assert_eq!(a.get().len(), b.get().len());
/// ```
#[derive(Clone)]
pub struct Source(Arc<dyn ByteSource>);

impl Source {
    /// The underlying source.
    #[must_use]
    pub fn get(&self) -> &Arc<dyn ByteSource> {
        &self.0
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Source")
            .field("len", &self.0.len())
            .field("in_memory", &self.0.as_slice().is_some())
            .finish()
    }
}

impl<S: ByteSource + 'static> From<S> for Source {
    fn from(source: S) -> Self {
        Self(Arc::new(source))
    }
}

impl From<Arc<dyn ByteSource>> for Source {
    fn from(source: Arc<dyn ByteSource>) -> Self {
        Self(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(source: &dyn ByteSource, offset: u64, len: usize) -> Vec<u8> {
        let mut buf = vec![0; len];
        let n = source.read_at(offset, &mut buf).unwrap();
        buf.truncate(n);
        buf
    }

    #[test]
    fn vec_source_reads_a_range() {
        assert_eq!(read(&b"abcdef".to_vec(), 2, 3), b"cde");
    }

    #[test]
    fn arc_source_reads_short_at_the_end() {
        let source: Arc<[u8]> = Arc::from(&b"abcdef"[..]);
        assert_eq!(read(&source, 4, 10), b"ef");
    }

    #[test]
    fn static_source_reads_nothing_past_the_end() {
        let source: &'static [u8] = b"abc";
        assert_eq!(read(&source, 99, 4), b"");
    }

    #[test]
    fn in_memory_sources_expose_their_slice() {
        let source: &'static [u8] = b"abc";
        assert_eq!(ByteSource::as_slice(&source), Some(&b"abc"[..]));
    }

    #[test]
    fn empty_vec_source_is_empty() {
        assert!(ByteSource::is_empty(&Vec::<u8>::new()));
    }

    #[test]
    fn len_counts_bytes() {
        let source: Arc<[u8]> = Arc::from(&b"abcdef"[..]);
        assert_eq!(ByteSource::len(&source), 6);
    }
}
