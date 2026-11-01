//! The input a [`Document`](crate::Document) owns: [`ByteSource`] ([ADR 0005] §2).
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md

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

/// Reads that span at most this many windows go through the cache (an object across a window
/// boundary, a scan chunk with its overlap). Longer ones (a large stream, a window grown for
/// one object) are read past it, so they don't evict the windows everything else uses.
const CACHED_SPAN: usize = 4;

/// Missing ranges one call collects before it stops reading and reports them (1 024 windows,
/// 64 MiB): a source whose `len()` claims far more than it has costs a bounded number of
/// `read_at` calls per round, not one per window it claims.
const MAX_PENDING: usize = 1024;

/// A shared [`ByteSource`]: what [`Document::open`](crate::Document::open) takes.
///
/// Every [`ByteSource`] converts into one (`Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`, or your
/// own type), and so does an `Arc<dyn ByteSource>`. Cloning a `Source` is cheap and shares
/// everything, including the window cache of a source that has no
/// [`as_slice`](ByteSource::as_slice): documents opened on several threads (ADR 0005 §7) from
/// clones of one `Source` reuse the windows each other read while they stay cached (two
/// threads missing the same window at once may both read it).
///
/// A source without `as_slice` is never read whole: the reader asks for 64 KiB windows
/// around the offsets it needs (grown while an object runs past the window's end, up to
/// [`Limits::max_read_window`](crate::Limits::max_read_window)), keeps the 64 most recently
/// used, checks xref entries with small reads, and reports the ranges a source answers
/// [`SourceError::Pending`] for as [`Error::DataPending`](crate::Error::DataPending).
///
/// ```
/// use std::sync::Arc;
/// use laminapdf_cos::{ByteSource, Source};
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
    /// Window `index`; `touch` marks it as just used (one-off reads don't, so they can't keep
    /// a window alive that nothing else needs).
    fn get(&mut self, index: usize, touch: bool) -> Option<Arc<[u8]>> {
        self.tick += 1;
        let tick = self.tick;
        self.windows.get_mut(&index).map(|(bytes, used)| {
            if touch {
                *used = tick;
            }
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

/// How [`Source::read`] treats the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fetch {
    /// Read missing windows whole and keep them (for reads of at most [`CACHED_SPAN`]
    /// windows; longer ones behave like [`Fetch::Exact`]).
    Cached,
    /// Use cached windows, read only the bytes asked for, keep nothing: for one-off looks
    /// (probes) and long spans.
    Exact,
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

    /// The input offset after the last byte.
    pub(crate) fn end(&self) -> usize {
        self.start + self.bytes().len()
    }

    /// Whether the window runs to the end of the input.
    pub(crate) const fn at_end(&self) -> bool {
        self.at_end
    }

    /// The bytes, owned (no copy when they already are).
    pub(crate) fn into_vec(self) -> Vec<u8> {
        match self.bytes {
            Bytes::Owned(v) => v,
            Bytes::Borrowed(b) => b.to_vec(),
            Bytes::Shared(b) => b.to_vec(),
        }
    }

    /// This window followed by (a copy of) `next`, which must start where this one ends.
    pub(crate) fn append(self, next: &Self) -> Self {
        let (start, at_end) = (self.start, next.at_end);
        let mut bytes = self.into_vec();
        bytes.extend_from_slice(next.bytes());
        Window {
            start,
            bytes: Bytes::Owned(bytes),
            at_end,
        }
    }

    /// `before` followed by this window; `before` must end where this one starts.
    pub(crate) fn prepend(self, before: Self) -> Self {
        let (start, at_end) = (before.start, self.at_end);
        let mut bytes = before.into_vec();
        bytes.extend_from_slice(self.bytes());
        Window {
            start,
            bytes: Bytes::Owned(bytes),
            at_end,
        }
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

    /// Whether enough is pending for one round ([`MAX_PENDING`] ranges): callers stop
    /// reading.
    pub(crate) const fn is_full(&self) -> bool {
        self.0.len() >= MAX_PENDING
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

/// Sorts `(offset, len)` ranges and merges those that overlap or touch (the form
/// [`Error::DataPending`] reports them in).
#[must_use]
pub fn merge_ranges(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
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

/// A source length as an offset on this target, or an error when the target can't address
/// all of it (a file over 4 GiB on wasm32): reading a prefix as if it were the whole file
/// would give wrong results.
fn addressable(len: u64, max: usize) -> CosResult<usize> {
    usize::try_from(len)
        .ok()
        .filter(|&len| len <= max)
        .ok_or(Error::InvalidStructure {
            kind: "source",
            message: "the input is larger than this target can address",
        })
}

/// Fills `buf` from `offset` with as many `read_at` calls as it takes (a source that claims
/// more bytes than `buf` has room for counts as having read only what fits).
///
/// # Errors
/// The source's errors ([`Error::DataPending`] for [`SourceError::Pending`]);
/// [`SourceError::Failed`] when the source ends before `buf` is full.
pub fn read_exact(source: &dyn ByteSource, offset: u64, buf: &mut [u8]) -> CosResult<()> {
    let mut done = 0;
    while let Some(rest) = buf.get_mut(done..).filter(|rest| !rest.is_empty()) {
        let at = offset.saturating_add(to_u64(done));
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

/// One window's share of a [`Source::read`].
enum Part {
    /// A whole cached window.
    Window(usize, Arc<[u8]>),
    /// Bytes read just for this call.
    Read(Vec<u8>),
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

    /// The input length.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when the input is larger than this target can address.
    pub(crate) fn len(&self) -> CosResult<usize> {
        match self.in_memory() {
            Some(slice) => Ok(slice.len()),
            None => addressable(self.0.source.len(), usize::MAX),
        }
    }

    fn cache(&self) -> MutexGuard<'_, Cache> {
        // A panic while the lock was held can only have left a stale entry out, never a
        // wrong one: every entry is inserted whole.
        self.0.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Window `index` read whole and cached. The lock is not held while reading, so two
    /// threads missing the same window may both read it.
    fn read_window(&self, index: usize, total: usize) -> CosResult<Arc<[u8]>> {
        let start = index.saturating_mul(WINDOW);
        let mut buf = vec![0; total.saturating_sub(start).min(WINDOW)];
        read_exact(self.0.source.as_ref(), to_u64(start), &mut buf)?;
        let bytes: Arc<[u8]> = buf.into();
        self.cache().insert(index, Arc::clone(&bytes));
        Ok(bytes)
    }

    /// Input bytes `start..start + len`, cut at the end of the input. An in-memory input
    /// returns all of itself instead, without copying. An empty range reads nothing.
    ///
    /// Cached windows are always used. Missing ones are read and kept with [`Fetch::Cached`]
    /// when the range spans at most [`CACHED_SPAN`] windows; otherwise only the bytes asked
    /// for are read, and nothing is kept.
    ///
    /// # Errors
    /// [`Error::DataPending`] with the ranges the source does not have yet (up to
    /// [`MAX_PENDING`] of them); [`Error::Source`] when it fails; [`Error::InvalidStructure`]
    /// as [`Source::len`].
    pub(crate) fn read(&self, start: usize, len: usize, fetch: Fetch) -> CosResult<Window<'_>> {
        if let Some(slice) = self.in_memory() {
            return Ok(Window {
                start: 0,
                bytes: Bytes::Borrowed(slice),
                at_end: true,
            });
        }
        let total = self.len()?;
        let start = start.min(total);
        let end = start.saturating_add(len).min(total);
        let at_end = end == total;
        if start == end {
            return Ok(Window {
                start,
                bytes: Bytes::Owned(Vec::new()),
                at_end,
            });
        }
        let windows = start / WINDOW..end.div_ceil(WINDOW);
        let keep = fetch == Fetch::Cached && windows.len() <= CACHED_SPAN;
        let mut parts = Vec::new();
        let mut pending = Pending::default();
        for index in windows {
            let window_start = index * WINDOW;
            let (from, to) = (start.max(window_start), end.min(window_start + WINDOW));
            // Bound first: a guard in the `match` scrutinee would be held through the arms,
            // and `read_window` takes the lock again.
            let cached = self.cache().get(index, keep);
            let part = match cached {
                Some(bytes) => Ok(Part::Window(window_start, bytes)),
                None if keep => self.read_window(index, total).map(|b| Part::Window(window_start, b)),
                None => {
                    let mut buf = vec![0; to - from];
                    read_exact(self.0.source.as_ref(), to_u64(from), &mut buf).map(|()| Part::Read(buf))
                }
            };
            match part {
                Ok(part) if pending.is_empty() => parts.push(part),
                Ok(_) => {}
                Err(e) => {
                    pending.absorb(e)?;
                    if pending.is_full() {
                        break;
                    }
                }
            }
        }
        pending.finish()?;
        Ok(Window {
            start,
            bytes: assemble(parts, start, end),
            at_end,
        })
    }

    /// Reads (and caches) the windows holding each `(offset, len)` range, so that every range
    /// a source is still missing is reported in one [`Error::DataPending`].
    ///
    /// # Errors
    /// As [`Source::read`].
    pub(crate) fn prefetch(&self, ranges: &[(usize, usize)]) -> CosResult<()> {
        if self.in_memory().is_some() {
            return Ok(());
        }
        let mut pending = Pending::default();
        // `read` caches only spans of at most `CACHED_SPAN` windows, so a longer range is
        // read in chunks of that size; otherwise its bytes would be fetched and dropped.
        let chunk = WINDOW * CACHED_SPAN;
        for &(start, len) in ranges {
            let end = start.saturating_add(len);
            let mut at = start;
            while at < end {
                let next = (at - at % WINDOW).saturating_add(chunk).min(end);
                if let Err(e) = self.read(at, next - at, Fetch::Cached) {
                    pending.absorb(e)?;
                }
                at = next;
            }
        }
        pending.finish()
    }

    /// The whole input: borrowed when it is in memory, otherwise copied from cached windows
    /// and read directly for the rest, without filling the cache. The buffer grows only as
    /// bytes arrive, and a source missing much of its input is asked for the first
    /// [`MAX_PENDING`] missing windows per round. Only for callers whose output holds the
    /// whole input anyway (the incremental writer).
    ///
    /// # Errors
    /// As [`Source::read`].
    pub(crate) fn whole(&self) -> CosResult<Cow<'_, [u8]>> {
        if let Some(slice) = self.in_memory() {
            return Ok(Cow::Borrowed(slice));
        }
        let total = self.len()?;
        Ok(Cow::Owned(self.read(0, total, Fetch::Exact)?.into_vec()))
    }
}

/// The bytes `start..end` out of consecutive parts: a cached window itself when it is exactly
/// that range, the bytes read when they are the only part, otherwise a copy.
fn assemble(mut parts: Vec<Part>, start: usize, end: usize) -> Bytes<'static> {
    if let [Part::Window(offset, bytes)] = parts.as_slice()
        && *offset == start
        && offset + <[u8]>::len(bytes) == end
    {
        return Bytes::Shared(Arc::clone(bytes));
    }
    if let [Part::Read(_)] = parts.as_slice()
        && let Some(Part::Read(bytes)) = parts.pop()
    {
        return Bytes::Owned(bytes);
    }
    let mut out = Vec::new();
    for part in &parts {
        match part {
            Part::Window(offset, bytes) => {
                let from = start.saturating_sub(*offset);
                let to = end.saturating_sub(*offset).min(<[u8]>::len(bytes));
                out.extend_from_slice(bytes.get(from..to).unwrap_or_default());
            }
            Part::Read(bytes) => out.extend_from_slice(bytes),
        }
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
        let window = source.read(WINDOW - 10, 20, Fetch::Cached).unwrap();
        assert_eq!(
            (window.start(), window.bytes(), window.at_end(), inner.reads()),
            (WINDOW - 10, &data[WINDOW - 10..WINDOW + 10], false, 2)
        );
    }

    #[test]
    fn a_window_that_is_exactly_one_cached_window_is_shared_not_copied() {
        let (source, _) = NoSlice::source(numbered(2 * WINDOW));
        let first = source.read(WINDOW, WINDOW, Fetch::Cached).unwrap();
        let second = source.read(WINDOW, WINDOW, Fetch::Cached).unwrap();
        assert!(std::ptr::eq(first.bytes(), second.bytes()));
    }

    #[test]
    fn a_prefetch_longer_than_the_cached_span_is_kept_for_later_reads() {
        let windows = 2 * CACHED_SPAN + 1;
        let (source, inner) = NoSlice::source(numbered(windows * WINDOW));
        source.prefetch(&[(0, windows * WINDOW)]).unwrap();
        let after_prefetch = inner.reads();
        for w in 0..windows {
            source.read(w * WINDOW, WINDOW, Fetch::Cached).unwrap();
        }
        assert_eq!(inner.reads(), after_prefetch);
    }

    #[test]
    fn a_window_past_the_end_is_cut_and_marked_at_end() {
        let (source, _) = NoSlice::source(numbered(WINDOW + 5));
        let window = source.read(WINDOW, 4 * WINDOW, Fetch::Cached).unwrap();
        assert_eq!((window.bytes().len(), window.at_end()), (5, true));
    }

    #[test]
    fn cached_windows_are_not_read_again() {
        let (source, inner) = NoSlice::source(numbered(2 * WINDOW));
        source.read(0, 2 * WINDOW, Fetch::Cached).unwrap();
        source.read(10, 2 * WINDOW, Fetch::Cached).unwrap();
        assert_eq!(inner.reads(), 2);
    }

    #[test]
    fn the_cache_keeps_a_bounded_number_of_windows() {
        let (source, _) = NoSlice::source(numbered((CACHE_WINDOWS + 3) * WINDOW));
        for index in 0..CACHE_WINDOWS + 3 {
            source.read(index * WINDOW, WINDOW, Fetch::Cached).unwrap();
        }
        assert_eq!(source.cache().windows.len(), CACHE_WINDOWS);
    }

    #[test]
    fn the_least_recently_used_window_is_evicted_first() {
        let (source, inner) = NoSlice::source(numbered((CACHE_WINDOWS + 1) * WINDOW));
        for index in 0..CACHE_WINDOWS {
            source.read(index * WINDOW, 1, Fetch::Cached).unwrap();
        }
        source.read(0, 1, Fetch::Cached).unwrap(); // window 0 is now the most recently used
        source.read(CACHE_WINDOWS * WINDOW, 1, Fetch::Cached).unwrap(); // evicts window 1
        let before = inner.reads();
        source.read(0, 1, Fetch::Cached).unwrap();
        let after_0 = inner.reads();
        source.read(WINDOW, 1, Fetch::Cached).unwrap();
        assert_eq!((after_0 - before, inner.reads() - after_0), (0, 1));
    }

    #[test]
    fn exact_reads_only_the_range_and_does_not_cache_it() {
        let data = numbered(2 * WINDOW);
        let (source, _) = NoSlice::source(data.clone());
        let probe = source.read(100, 32, Fetch::Exact).unwrap();
        assert_eq!((probe.bytes(), source.cache().windows.len()), (&data[100..132], 0));
    }

    #[test]
    fn exact_uses_cached_windows_when_they_hold_the_range() {
        let (source, inner) = NoSlice::source(numbered(WINDOW));
        source.read(0, WINDOW, Fetch::Cached).unwrap();
        source.read(100, 32, Fetch::Exact).unwrap();
        assert_eq!(inner.reads(), 1);
    }

    #[test]
    fn an_in_memory_source_returns_all_of_itself_without_copying() {
        let data: Arc<[u8]> = numbered(100).into();
        let source = Source::from(Arc::clone(&data));
        let window = source.read(40, 10, Fetch::Cached).unwrap();
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
            source.read(0, 5 * WINDOW, Fetch::Cached).err(),
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

    #[test]
    fn an_empty_range_reads_nothing() {
        let (source, inner) = NoSlice::source(numbered(2 * WINDOW));
        let window = source.read(WINDOW + 7, 0, Fetch::Cached).unwrap();
        assert_eq!((window.bytes().len(), inner.reads()), (0, 0));
    }

    #[test]
    fn a_long_read_does_not_evict_the_cached_windows() {
        let (source, inner) = NoSlice::source(numbered((CACHE_WINDOWS + 10) * WINDOW));
        source.read(0, WINDOW, Fetch::Cached).unwrap();
        source
            .read(WINDOW, (CACHE_WINDOWS + 5) * WINDOW, Fetch::Cached)
            .unwrap();
        let before = inner.reads();
        source.read(0, WINDOW, Fetch::Cached).unwrap();
        assert_eq!((inner.reads() - before, source.cache().windows.len()), (0, 1));
    }

    #[test]
    fn an_exact_read_uses_the_cached_part_and_reads_only_the_rest() {
        let data = numbered(2 * WINDOW);
        let (source, inner) = NoSlice::source(data.clone());
        source.read(0, WINDOW, Fetch::Cached).unwrap();
        let before = inner.reads();
        let window = source.read(WINDOW - 10, 20, Fetch::Exact).unwrap();
        assert_eq!(
            (window.bytes(), inner.reads() - before),
            (&data[WINDOW - 10..WINDOW + 10], 1)
        );
    }

    #[test]
    fn whole_copies_cached_windows_and_reads_the_rest_without_caching() {
        let data = numbered(3 * WINDOW + 5);
        let (source, inner) = NoSlice::source(data.clone());
        source.read(WINDOW, WINDOW, Fetch::Cached).unwrap();
        let whole = source.whole().unwrap().into_owned();
        assert_eq!(
            (whole == data, inner.reads(), source.cache().windows.len()),
            (true, 4, 1)
        );
    }

    /// Claims a terabyte and answers `Pending` for all of it.
    struct PendingTerabyte(AtomicUsize);

    impl ByteSource for PendingTerabyte {
        fn len(&self) -> u64 {
            1 << 40
        }

        fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(SourceError::Pending {
                offset,
                len: u64::try_from(buf.len()).unwrap(),
            })
        }
    }

    #[test]
    fn whole_on_a_source_that_overstates_its_length_stops_after_one_round_of_ranges() {
        let inner = Arc::new(PendingTerabyte(AtomicUsize::new(0)));
        let source = Source::from(Arc::clone(&inner) as Arc<dyn ByteSource>);
        let Err(Error::DataPending { ranges }) = source.whole() else {
            panic!("expected DataPending");
        };
        assert_eq!(
            (inner.0.load(Ordering::Relaxed), ranges,),
            (MAX_PENDING, vec![(0, to_u64(MAX_PENDING * WINDOW))])
        );
    }

    #[test]
    fn a_length_over_what_the_target_addresses_is_an_error_not_a_prefix() {
        let wasm32_max = usize::try_from(u32::MAX).unwrap();
        assert_eq!(
            (
                addressable(5 << 30, wasm32_max).is_err(),
                addressable(5 << 20, wasm32_max)
            ),
            (true, Ok(5 << 20))
        );
    }
}
