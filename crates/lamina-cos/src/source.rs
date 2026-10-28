//! The input a [`Document`](crate::Document) owns: [`ByteSource`] ([ADR 0005] §2).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

use std::fmt;
use std::sync::{Arc, OnceLock};

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
/// own type), and so does an `Arc<dyn ByteSource>`. Cloning a `Source` is cheap and shares
/// everything, including the in-memory copy of a source that has no
/// [`as_slice`](ByteSource::as_slice): open one document per thread (ADR 0005 §7) from clones
/// of one `Source`, and the input is read and held once.
///
/// ```
/// use std::sync::Arc;
/// use lamina_cos::{ByteSource, Source};
///
/// let shared: Arc<dyn ByteSource> = Arc::new(b"%PDF-1.7".to_vec());
/// let a = Source::from(shared);
/// let b = a.clone();
/// assert!(Arc::ptr_eq(a.get(), b.get()));
/// ```
#[derive(Clone)]
pub struct Source(Arc<Shared>);

struct Shared {
    source: Arc<dyn ByteSource>,
    /// The whole input, read once, for a source without `as_slice`. Windowed reading over
    /// [`ByteSource::read_at`] (ADR 0005 §2) is later work.
    buffered: OnceLock<Arc<[u8]>>,
}

/// Bytes requested per [`ByteSource::read_at`] call when buffering a source.
const READ_CHUNK: usize = 64 * 1024;

impl Source {
    fn new(source: Arc<dyn ByteSource>) -> Self {
        Self(Arc::new(Shared {
            source,
            buffered: OnceLock::new(),
        }))
    }

    /// The underlying source.
    #[must_use]
    pub fn get(&self) -> &Arc<dyn ByteSource> {
        &self.0.source
    }

    /// The whole input in memory: the source's own slice, or a copy read once and shared by
    /// every clone of this `Source`.
    pub(crate) fn bytes(&self) -> Result<&[u8], SourceError> {
        if let Some(slice) = self.0.source.as_slice() {
            return Ok(slice);
        }
        if self.0.buffered.get().is_none() {
            let read = read_all(self.0.source.as_ref())?;
            // Another thread may have filled it meanwhile; either copy is the same bytes.
            let _ = self.0.buffered.set(read);
        }
        Ok(self.0.buffered.get().map_or(&[][..], |b| &**b))
    }

    /// [`Source::bytes`] after it has succeeded once (empty before that).
    pub(crate) fn loaded(&self) -> &[u8] {
        self.0
            .source
            .as_slice()
            .or_else(|| self.0.buffered.get().map(|b| &**b))
            .unwrap_or_default()
    }
}

/// Reads a whole source in bounded chunks into a growing buffer, so a `len()` that lies
/// costs nothing until the bytes actually arrive.
///
/// # Errors
/// The source's errors; [`SourceError::Failed`] when it ends before its reported length.
fn read_all(source: &dyn ByteSource) -> Result<Arc<[u8]>, SourceError> {
    let len = source.len();
    let mut out: Vec<u8> = Vec::new();
    let mut chunk = vec![0; READ_CHUNK];
    let mut offset: u64 = 0;
    while offset < len {
        let want = usize::try_from(len - offset).map_or(READ_CHUNK, |rest| rest.min(READ_CHUNK));
        let Some(buf) = chunk.get_mut(..want) else { break };
        let n = source.read_at(offset, buf)?;
        if n == 0 {
            return Err(SourceError::Failed {
                offset,
                len: len - offset,
            });
        }
        out.extend_from_slice(buf.get(..n).unwrap_or_default());
        offset = offset.saturating_add(u64::try_from(n).unwrap_or(u64::MAX));
    }
    Ok(out.into())
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Source")
            .field("len", &self.0.source.len())
            .field("in_memory", &self.0.source.as_slice().is_some())
            .field("buffered", &self.0.buffered.get().is_some())
            .finish()
    }
}

impl<S: ByteSource + 'static> From<S> for Source {
    fn from(source: S) -> Self {
        Self::new(Arc::new(source))
    }
}

impl From<Arc<dyn ByteSource>> for Source {
    fn from(source: Arc<dyn ByteSource>) -> Self {
        Self::new(source)
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

    /// Claims a huge length but holds only `data`.
    struct Liar(Vec<u8>);

    impl ByteSource for Liar {
        fn len(&self) -> u64 {
            1 << 40
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            Ok(read_slice(&self.0, offset, buf))
        }
    }

    #[test]
    fn read_all_fails_cleanly_when_a_source_ends_before_its_length() {
        assert_eq!(
            read_all(&Liar(b"abc".to_vec())),
            Err(SourceError::Failed {
                offset: 3,
                len: (1 << 40) - 3
            })
        );
    }

    #[test]
    fn read_all_reads_across_chunks() {
        struct NoSlice(Vec<u8>);
        impl ByteSource for NoSlice {
            fn len(&self) -> u64 {
                u64::try_from(self.0.len()).unwrap()
            }
            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
                Ok(read_slice(&self.0, offset, buf))
            }
        }
        let data: Vec<u8> = (0..READ_CHUNK * 2 + 5)
            .map(|i| u8::try_from(i % 251).unwrap())
            .collect();
        assert_eq!(read_all(&NoSlice(data.clone())).map(|b| b.to_vec()), Ok(data));
    }

    #[test]
    fn len_counts_bytes() {
        let source: Arc<[u8]> = Arc::from(&b"abcdef"[..]);
        assert_eq!(ByteSource::len(&source), 6);
    }
}
