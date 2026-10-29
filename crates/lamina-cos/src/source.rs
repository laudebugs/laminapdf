//! The input a [`Document`](crate::Document) owns: [`ByteSource`] ([ADR 0005] §2).
//!
//! [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::error::{Error, Result as CosResult};

/// Random-access input bytes: in memory, memory-mapped, a browser `Blob`, an HTTP range cache.
///
/// Synchronous by design (the core has no async runtime): a source whose bytes have not
/// arrived yet answers [`SourceError::Pending`], and the shell fetches them and retries.
/// In-memory sources return their bytes from [`ByteSource::as_slice`], which lets the
/// parser work on them in place.
///
/// A source without `as_slice` is read in aligned 64 KiB windows (and small probes for xref
/// entry checks); `read_at` may return fewer bytes than asked for and is called again for the
/// rest. A [`SourceError::Pending`] is reported to the caller as
/// [`Error::DataPending`](crate::Error::DataPending) with the range the source gave.
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

/// Bytes per window: the unit in which a source without [`ByteSource::as_slice`] is read,
/// cached and grown (ADR 0005 §2).
pub(crate) const WINDOW: usize = 64 * 1024;

/// Windows a [`Source`] keeps for reuse (4 MiB with [`WINDOW`]), shared by its clones. The
/// least recently used one is dropped first, so a hostile file can't make the cache hold the
/// whole input.
const CACHE_WINDOWS: usize = 64;

/// A shared [`ByteSource`]: what [`Document::open`](crate::Document::open) takes.
///
/// Every [`ByteSource`] converts into one (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`, or your
/// own type), and so does an `Arc<dyn ByteSource>`. Cloning a `Source` is cheap and shares
/// everything, including the window cache of a source that has no
/// [`as_slice`](ByteSource::as_slice): open one document per thread (ADR 0005 §7) from clones
/// of one `Source`, and each window of the input is read once while it stays cached.
///
/// A source without `as_slice` is never read whole: the reader asks for 64 KiB windows
/// around the offsets it needs (doubled while an object runs past the window's end), keeps
/// the 64 most recently used, and reports the ranges a source answers
/// [`SourceError::Pending`] for as [`Error::DataPending`](crate::Error::DataPending).
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
    /// Recently read windows of a source without `as_slice`.
    cache: Mutex<Cache>,
}

/// Windows by index (`offset / WINDOW`), each with the tick of its last use.
#[derive(Default)]
struct Cache {
    windows: BTreeMap<usize, (Arc<[u8]>, u64)>,
    tick: u64,
}

impl Cache {
    fn get(&mut self, index: usize) -> Option<Arc<[u8]>> {
        self.tick += 1;
        let tick = self.tick;
        self.windows.get_mut(&index).map(|(bytes, used)| {
            *used = tick;
            Arc::clone(bytes)
        })
    }

    fn insert(&mut self, index: usize, bytes: Arc<[u8]>) {
        self.tick += 1;
        self.windows.insert(index, (bytes, self.tick));
        while self.windows.len() > CACHE_WINDOWS {
            let oldest = self.windows.iter().min_by_key(|(_, (_, used))| *used).map(|(&i, _)| i);
            match oldest {
                Some(i) => self.windows.remove(&i),
                None => break,
            };
        }
    }
}

/// A run of input bytes and where it starts.
pub(crate) struct Window<'s> {
    start: usize,
    bytes: Bytes<'s>,
    at_end: bool,
}

enum Bytes<'s> {
    /// The whole in-memory input.
    Borrowed(&'s [u8]),
    /// One cached window, shared.
    Shared(Arc<[u8]>),
    /// Copied together from several windows, or read exactly.
    Owned(Vec<u8>),
}

impl Window<'_> {
    /// The bytes; `bytes()[0]` is input byte [`Window::start`].
    pub(crate) fn bytes(&self) -> &[u8] {
        match &self.bytes {
            Bytes::Borrowed(b) => b,
            Bytes::Shared(b) => b,
            Bytes::Owned(b) => b,
        }
    }

    /// The input offset of the first byte.
    pub(crate) const fn start(&self) -> usize {
        self.start
    }

    /// Whether the window runs to the end of the input.
    pub(crate) const fn at_end(&self) -> bool {
        self.at_end
    }
}

/// Ranges sources answered [`SourceError::Pending`] for, collected so the shell can fetch
/// them all in one round.
#[derive(Debug, Default)]
pub(crate) struct Pending(Vec<(u64, u64)>);

impl Pending {
    /// Keeps the ranges of an [`Error::DataPending`]; any other error is returned.
    pub(crate) fn absorb(&mut self, e: Error) -> CosResult<()> {
        match e {
            Error::DataPending { ranges } => {
                self.0.extend(ranges);
                Ok(())
            }
            other => Err(other),
        }
    }

    /// Whether nothing is pending.
    pub(crate) const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `Ok` when nothing is pending, else [`Error::DataPending`] with the ranges sorted and
    /// overlapping or adjacent ones merged.
    pub(crate) fn finish(self) -> CosResult<()> {
        if self.0.is_empty() {
            return Ok(());
        }
        Err(self.into_error())
    }

    /// [`Error::DataPending`] with the ranges sorted and merged.
    pub(crate) fn into_error(self) -> Error {
        Error::DataPending {
            ranges: merge_ranges(self.0),
        }
    }
}

/// Sorts `(offset, len)` ranges and merges those that overlap or touch.
fn merge_ranges(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (offset, len) in ranges {
        let end = offset.saturating_add(len);
        match merged.last_mut() {
            Some((start, run)) if offset <= start.saturating_add(*run) => {
                *run = end.max(start.saturating_add(*run)) - *start;
            }
            _ => merged.push((offset, len)),
        }
    }
    merged
}

fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Fills `buf` from `offset` with as many `read_at` calls as it takes.
///
/// # Errors
/// The source's errors ([`Error::DataPending`] for [`SourceError::Pending`]);
/// [`SourceError::Failed`] when the source ends before `buf` is full.
fn read_exact(source: &dyn ByteSource, offset: usize, buf: &mut [u8]) -> CosResult<()> {
    let mut done = 0;
    while let Some(rest) = buf.get_mut(done..).filter(|rest| !rest.is_empty()) {
        let at = to_u64(offset + done);
        let n = source.read_at(at, rest)?.min(rest.len());
        if n == 0 {
            return Err(Error::Source(SourceError::Failed {
                offset: at,
                len: to_u64(rest.len()),
            }));
        }
        done += n;
    }
    Ok(())
}

impl Source {
    fn new(source: Arc<dyn ByteSource>) -> Self {
        Self(Arc::new(Shared {
            source,
            cache: Mutex::new(Cache::default()),
        }))
    }

    /// The underlying source.
    #[must_use]
    pub fn get(&self) -> &Arc<dyn ByteSource> {
        &self.0.source
    }

    /// The whole input, when the source holds it in memory ([`ByteSource::as_slice`]).
    pub(crate) fn in_memory(&self) -> Option<&[u8]> {
        self.0.source.as_slice()
    }

    /// The input length (addressable part of it, on targets with 32-bit offsets).
    pub(crate) fn len(&self) -> usize {
        match self.in_memory() {
            Some(slice) => slice.len(),
            None => usize::try_from(self.0.source.len()).unwrap_or(usize::MAX),
        }
    }

    fn cache(&self) -> MutexGuard<'_, Cache> {
        // A panic while the lock was held can only have left a stale entry out, never a
        // wrong one: every entry is inserted whole.
        self.0.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Window `index` (`[index * WINDOW, …)`, shorter at the end of the input), from the cache
    /// or read and cached.
    fn window_at(&self, index: usize) -> CosResult<Arc<[u8]>> {
        if let Some(bytes) = self.cache().get(index) {
            return Ok(bytes);
        }
        let start = index.saturating_mul(WINDOW);
        let len = self.len().saturating_sub(start).min(WINDOW);
        let mut buf = vec![0; len];
        read_exact(self.0.source.as_ref(), start, &mut buf)?;
        let bytes: Arc<[u8]> = buf.into();
        self.cache().insert(index, Arc::clone(&bytes));
        Ok(bytes)
    }

    /// Input bytes `start..start + len` (cut at the end of the input), read in cached
    /// [`WINDOW`]s. An in-memory input returns all of itself instead, without copying.
    ///
    /// # Errors
    /// [`Error::DataPending`] with every window range the source does not have yet;
    /// [`Error::Source`] when it fails.
    pub(crate) fn window(&self, start: usize, len: usize) -> CosResult<Window<'_>> {
        if let Some(slice) = self.in_memory() {
            return Ok(Window {
                start: 0,
                bytes: Bytes::Borrowed(slice),
                at_end: true,
            });
        }
        let total = self.len();
        let start = start.min(total);
        let end = start.saturating_add(len).min(total);
        let mut pending = Pending::default();
        let mut parts = Vec::new();
        for index in start / WINDOW..end.div_ceil(WINDOW) {
            match self.window_at(index) {
                Ok(bytes) if pending.is_empty() => parts.push((index * WINDOW, bytes)),
                Ok(_) => {}
                Err(e) => pending.absorb(e)?,
            }
        }
        pending.finish()?;
        Ok(Window {
            start,
            bytes: assemble(&parts, start, end),
            at_end: end == total,
        })
    }

    /// Input bytes `start..start + len` (cut at the end of the input) for a one-off look:
    /// from cached windows when they hold them, otherwise read as exactly that range and not
    /// cached. An in-memory input returns all of itself.
    ///
    /// # Errors
    /// As [`Source::window`].
    pub(crate) fn exact(&self, start: usize, len: usize) -> CosResult<Window<'_>> {
        if let Some(slice) = self.in_memory() {
            return Ok(Window {
                start: 0,
                bytes: Bytes::Borrowed(slice),
                at_end: true,
            });
        }
        let total = self.len();
        let start = start.min(total);
        let end = start.saturating_add(len).min(total);
        let cached: Option<Vec<_>> = {
            let mut cache = self.cache();
            (start / WINDOW..end.div_ceil(WINDOW))
                .map(|index| cache.get(index).map(|bytes| (index * WINDOW, bytes)))
                .collect()
        };
        let bytes = if let Some(parts) = cached {
            assemble(&parts, start, end)
        } else {
            let mut buf = vec![0; end - start];
            read_exact(self.0.source.as_ref(), start, &mut buf)?;
            Bytes::Owned(buf)
        };
        Ok(Window {
            start,
            bytes,
            at_end: end == total,
        })
    }

    /// Reads (and caches) the windows holding each `(offset, len)` range, so that every range
    /// a source is still missing is reported in one [`Error::DataPending`].
    ///
    /// # Errors
    /// As [`Source::window`].
    pub(crate) fn prefetch(&self, ranges: &[(usize, usize)]) -> CosResult<()> {
        if self.in_memory().is_some() {
            return Ok(());
        }
        let mut pending = Pending::default();
        for &(start, len) in ranges {
            if let Err(e) = self.window(start, len) {
                pending.absorb(e)?;
            }
        }
        pending.finish()
    }

    /// The whole input: borrowed when it is in memory, otherwise read window by window into
    /// one buffer (which grows only as bytes arrive, so a `len()` that lies costs nothing).
    /// Only for callers whose output holds the whole input anyway (the incremental writer).
    ///
    /// # Errors
    /// As [`Source::window`]; every missing window is reported at once.
    pub(crate) fn whole(&self) -> CosResult<Cow<'_, [u8]>> {
        if let Some(slice) = self.in_memory() {
            return Ok(Cow::Borrowed(slice));
        }
        let mut out = Vec::new();
        let mut pending = Pending::default();
        for index in 0..self.len().div_ceil(WINDOW) {
            match self.window_at(index) {
                Ok(bytes) if pending.is_empty() => out.extend_from_slice(&bytes),
                Ok(_) => {}
                Err(e) => pending.absorb(e)?,
            }
        }
        pending.finish()?;
        Ok(Cow::Owned(out))
    }
}

/// The bytes `start..end` out of consecutive windows `(offset, bytes)`: the window itself when
/// it is exactly that range, otherwise a copy.
fn assemble(parts: &[(usize, Arc<[u8]>)], start: usize, end: usize) -> Bytes<'static> {
    if let [(offset, bytes)] = parts
        && *offset == start
        && offset + <[u8]>::len(bytes) == end
    {
        return Bytes::Shared(Arc::clone(bytes));
    }
    let mut out = Vec::with_capacity(end - start);
    for (offset, bytes) in parts {
        let from = start.saturating_sub(*offset);
        let to = end.saturating_sub(*offset).min(<[u8]>::len(bytes));
        out.extend_from_slice(bytes.get(from..to).unwrap_or_default());
    }
    Bytes::Owned(out)
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Source")
            .field("len", &self.0.source.len())
            .field("in_memory", &self.0.source.as_slice().is_some())
            .field("cached_windows", &self.cache().windows.len())
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
pub(crate) mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A source without `as_slice` over `data`, counting its `read_at` calls.
    pub(crate) struct NoSlice {
        data: Vec<u8>,
        reads: AtomicUsize,
    }

    impl NoSlice {
        pub(crate) fn source(data: impl Into<Vec<u8>>) -> (Source, Arc<Self>) {
            let inner = Arc::new(Self {
                data: data.into(),
                reads: AtomicUsize::new(0),
            });
            (Source::from(Arc::clone(&inner) as Arc<dyn ByteSource>), inner)
        }

        pub(crate) fn reads(&self) -> usize {
            self.reads.load(Ordering::Relaxed)
        }
    }

    impl ByteSource for NoSlice {
        fn len(&self) -> u64 {
            u64::try_from(self.data.len()).unwrap()
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            Ok(read_slice(&self.data, offset, buf))
        }
    }

    /// `len` bytes counting up, so every offset is recognisable.
    pub(crate) fn numbered(len: usize) -> Vec<u8> {
        (0..len).map(|i| u8::try_from(i % 251).unwrap()).collect()
    }

    #[test]
    fn window_reads_whole_aligned_windows_and_cuts_the_requested_range() {
        let data = numbered(3 * WINDOW);
        let (source, inner) = NoSlice::source(data.clone());
        let window = source.window(WINDOW - 10, 20).unwrap();
        assert_eq!(
            (window.start(), window.bytes(), window.at_end(), inner.reads()),
            (WINDOW - 10, &data[WINDOW - 10..WINDOW + 10], false, 2)
        );
    }

    #[test]
    fn a_window_that_is_exactly_one_cached_window_is_shared_not_copied() {
        let (source, _) = NoSlice::source(numbered(2 * WINDOW));
        let first = source.window(WINDOW, WINDOW).unwrap();
        let second = source.window(WINDOW, WINDOW).unwrap();
        assert!(std::ptr::eq(first.bytes(), second.bytes()));
    }

    #[test]
    fn a_window_past_the_end_is_cut_and_marked_at_end() {
        let (source, _) = NoSlice::source(numbered(WINDOW + 5));
        let window = source.window(WINDOW, 4 * WINDOW).unwrap();
        assert_eq!((window.bytes().len(), window.at_end()), (5, true));
    }

    #[test]
    fn cached_windows_are_not_read_again() {
        let (source, inner) = NoSlice::source(numbered(2 * WINDOW));
        source.window(0, 2 * WINDOW).unwrap();
        source.window(10, 2 * WINDOW).unwrap();
        assert_eq!(inner.reads(), 2);
    }

    #[test]
    fn the_cache_keeps_a_bounded_number_of_windows() {
        let (source, _) = NoSlice::source(numbered((CACHE_WINDOWS + 3) * WINDOW));
        source.whole().unwrap();
        assert_eq!(source.cache().windows.len(), CACHE_WINDOWS);
    }

    #[test]
    fn the_least_recently_used_window_is_evicted_first() {
        let (source, inner) = NoSlice::source(numbered((CACHE_WINDOWS + 1) * WINDOW));
        for index in 0..CACHE_WINDOWS {
            source.window(index * WINDOW, 1).unwrap();
        }
        source.window(0, 1).unwrap(); // window 0 is now the most recently used
        source.window(CACHE_WINDOWS * WINDOW, 1).unwrap(); // evicts window 1
        let before = inner.reads();
        source.window(0, 1).unwrap();
        let after_0 = inner.reads();
        source.window(WINDOW, 1).unwrap();
        assert_eq!((after_0 - before, inner.reads() - after_0), (0, 1));
    }

    #[test]
    fn exact_reads_only_the_range_and_does_not_cache_it() {
        let data = numbered(2 * WINDOW);
        let (source, _) = NoSlice::source(data.clone());
        let probe = source.exact(100, 32).unwrap();
        assert_eq!((probe.bytes(), source.cache().windows.len()), (&data[100..132], 0));
    }

    #[test]
    fn exact_uses_cached_windows_when_they_hold_the_range() {
        let (source, inner) = NoSlice::source(numbered(WINDOW));
        source.window(0, WINDOW).unwrap();
        source.exact(100, 32).unwrap();
        assert_eq!(inner.reads(), 1);
    }

    #[test]
    fn an_in_memory_source_returns_all_of_itself_without_copying() {
        let data: Arc<[u8]> = numbered(100).into();
        let source = Source::from(Arc::clone(&data));
        let window = source.window(40, 10).unwrap();
        assert!(window.start() == 0 && std::ptr::eq(window.bytes(), data.as_ref()));
    }

    /// Answers `Pending` for every read of window 1 and window 3.
    struct Holes(Vec<u8>);

    impl ByteSource for Holes {
        fn len(&self) -> u64 {
            u64::try_from(self.0.len()).unwrap()
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            let index = usize::try_from(offset).unwrap() / WINDOW;
            if index == 1 || index == 3 {
                return Err(SourceError::Pending {
                    offset,
                    len: u64::try_from(buf.len()).unwrap(),
                });
            }
            Ok(read_slice(&self.0, offset, buf))
        }
    }

    #[test]
    fn a_window_reports_every_missing_part_at_once() {
        let source = Source::from(Holes(numbered(5 * WINDOW)));
        let w = to_u64(WINDOW);
        assert_eq!(
            source.window(0, 5 * WINDOW).err(),
            Some(Error::DataPending {
                ranges: vec![(w, w), (3 * w, w)]
            })
        );
    }

    #[test]
    fn whole_reads_every_window_or_reports_the_missing_ones() {
        let source = Source::from(Holes(numbered(5 * WINDOW)));
        let w = to_u64(WINDOW);
        assert_eq!(
            source.whole().err(),
            Some(Error::DataPending {
                ranges: vec![(w, w), (3 * w, w)]
            })
        );
    }

    #[test]
    fn merge_ranges_sorts_and_joins_overlapping_and_adjacent_ranges() {
        assert_eq!(
            merge_ranges(vec![(50, 10), (0, 10), (10, 5), (12, 1), (58, 4), (100, 1)]),
            vec![(0, 15), (50, 12), (100, 1)]
        );
    }

    #[test]
    fn read_exact_fails_cleanly_when_a_source_ends_before_its_length() {
        /// Claims a terabyte, holds three bytes.
        struct Liar;
        impl ByteSource for Liar {
            fn len(&self) -> u64 {
                1 << 40
            }
            fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
                Ok(read_slice(b"abc", offset, buf))
            }
        }
        let mut buf = [0; 10];
        assert_eq!(
            read_exact(&Liar, 0, &mut buf),
            Err(Error::Source(SourceError::Failed { offset: 3, len: 7 }))
        );
    }

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
