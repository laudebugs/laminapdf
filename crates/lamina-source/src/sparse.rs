//! [`SparseSource`]: a [`ByteSource`] that starts empty and is filled range by range.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::{Arc, Mutex, MutexGuard};

use lamina_cos::{ByteSource, SourceError};

/// Why [`SparseSource::supply`] refused a chunk.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SupplyError {
    /// The chunk reaches past the end of the source.
    #[error("bytes {offset}..+{len} are past the end of a {size}-byte source")]
    OutOfRange {
        /// First byte of the chunk.
        offset: u64,
        /// Length of the chunk.
        len: u64,
        /// The source's length.
        size: u64,
    },
    /// Holding the chunk would exceed the source's byte budget.
    #[error("holding {needed} bytes would exceed the {budget}-byte budget")]
    Budget {
        /// Bytes the source would hold with the chunk.
        needed: u64,
        /// The budget.
        budget: u64,
    },
    /// A thread panicked while it held the chunk map.
    #[error("the source is unusable after an earlier panic")]
    Poisoned,
}

/// A [`ByteSource`] of known length whose bytes arrive later, chunk by chunk.
///
/// The reader asks for windows with [`ByteSource::read_at`]; a window the shell hasn't supplied
/// yet answers [`SourceError::Pending`], which the reader reports as `data-pending` with the
/// ranges it needs. The shell fetches those ranges (HTTP `Range`, a file, a callback),
/// [`supply`](Self::supply)s them and retries the call; [`retry`](crate::retry) runs that loop.
///
/// Chunks are `Arc<[u8]>`, merged when they touch, and their total is bounded by a byte budget.
/// Supplied bytes are never evicted: the document may already have read them. No I/O, clocks
/// or threads; the chunk map is behind a lock because `supply` runs while the document shares
/// the source, which is why this lives in an edge crate (ADR 0005 §2, §7).
#[derive(Debug)]
pub struct SparseSource {
    len: u64,
    budget: u64,
    chunks: Mutex<Chunks>,
}

/// Supplied chunks by first byte; never overlapping or touching (merged on supply).
#[derive(Debug, Default)]
struct Chunks {
    map: BTreeMap<u64, Arc<[u8]>>,
    held: u64,
}

impl SparseSource {
    /// An empty source of `len` bytes with no budget (it may come to hold the whole file).
    pub fn new(len: u64) -> Self {
        Self::with_budget(len, u64::MAX)
    }

    /// An empty source of `len` bytes that holds at most `budget` bytes.
    pub fn with_budget(len: u64, budget: u64) -> Self {
        Self {
            len,
            budget,
            chunks: Mutex::new(Chunks::default()),
        }
    }

    /// The byte budget.
    pub fn budget(&self) -> u64 {
        self.budget
    }

    /// Bytes supplied so far (overlaps counted once).
    ///
    /// # Errors
    /// [`SupplyError::Poisoned`] after a panic while the map was locked.
    pub fn held(&self) -> Result<u64, SupplyError> {
        Ok(self.lock()?.held)
    }

    /// Adds `bytes` at `offset`. Bytes already held win over overlapping new ones (the document
    /// may have read them); chunks that touch are merged into one.
    ///
    /// # Errors
    /// [`SupplyError::OutOfRange`] past the end, [`SupplyError::Budget`] over the budget
    /// (nothing is added then).
    pub fn supply(&self, offset: u64, bytes: &[u8]) -> Result<(), SupplyError> {
        let len = bytes.len() as u64;
        let end = offset
            .checked_add(len)
            .filter(|&e| e <= self.len)
            .ok_or(SupplyError::OutOfRange {
                offset,
                len,
                size: self.len,
            })?;
        if len == 0 {
            return Ok(());
        }
        let mut chunks = self.lock()?;
        // Every chunk that overlaps or touches [offset, end) is merged with the new bytes.
        let touching: Vec<(u64, Arc<[u8]>)> = chunks
            .map
            .range(..=end)
            .filter(|(start, c)| *start + span(c) >= offset)
            .map(|(s, c)| (*s, Arc::clone(c)))
            .collect();
        let start = touching.first().map_or(offset, |(s, _)| (*s).min(offset));
        let stop = touching.last().map_or(end, |(s, c)| (s + span(c)).max(end));
        let old: u64 = touching.iter().map(|(_, c)| span(c)).sum();
        let needed = chunks.held - old + (stop - start);
        if needed > self.budget {
            return Err(SupplyError::Budget {
                needed,
                budget: self.budget,
            });
        }
        let mut merged = vec![
            0_u8;
            usize::try_from(stop - start).map_err(|_| SupplyError::Budget {
                needed,
                budget: self.budget,
            })?
        ];
        copy_into(&mut merged, start, offset, bytes);
        for (s, c) in &touching {
            copy_into(&mut merged, start, *s, c);
            chunks.map.remove(s);
        }
        chunks.map.insert(start, Arc::from(merged));
        chunks.held = needed;
        Ok(())
    }

    /// The parts of `ranges` not supplied yet, clamped to the source and in order, adjacent
    /// gaps merged: what a fetch must still bring.
    ///
    /// # Errors
    /// [`SupplyError::Poisoned`] after a panic while the map was locked.
    pub fn missing(&self, ranges: &[Range<u64>]) -> Result<Vec<Range<u64>>, SupplyError> {
        let chunks = self.lock()?;
        let mut wanted: Vec<Range<u64>> = ranges
            .iter()
            .map(|r| r.start.min(self.len)..r.end.min(self.len))
            .filter(|r| r.start < r.end)
            .collect();
        wanted.sort_by_key(|r| r.start);
        let mut out: Vec<Range<u64>> = Vec::new();
        for r in wanted {
            let mut at = r.start;
            while at < r.end {
                if let Some(held_to) = chunks.covering(at) {
                    at = held_to.min(r.end);
                } else {
                    let gap_end = chunks.next_start(at).map_or(r.end, |n| n.min(r.end));
                    push_merged(&mut out, at..gap_end);
                    at = gap_end;
                }
            }
        }
        Ok(out)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Chunks>, SupplyError> {
        self.chunks.lock().map_err(|_| SupplyError::Poisoned)
    }
}

impl Chunks {
    /// The end of the chunk holding byte `at`, if one does.
    fn covering(&self, at: u64) -> Option<u64> {
        let (start, c) = self.map.range(..=at).next_back()?;
        let end = start + span(c);
        (at < end).then_some(end)
    }

    /// The first byte of the first chunk after `at`.
    fn next_start(&self, at: u64) -> Option<u64> {
        self.map.range(at.saturating_add(1)..).next().map(|(s, _)| *s)
    }
}

/// A chunk's length in bytes (the slice's; `Arc<[u8]>` is also a `ByteSource`, whose `len`
/// would otherwise be picked).
fn span(c: &[u8]) -> u64 {
    c.len() as u64
}

/// Copies `bytes` (which start at absolute `at`) into `buf` (which starts at absolute `base`).
fn copy_into(buf: &mut [u8], base: u64, at: u64, bytes: &[u8]) {
    let from = usize::try_from(at - base).unwrap_or(usize::MAX);
    if let Some(dst) = buf.get_mut(from..).and_then(|d| d.get_mut(..bytes.len())) {
        dst.copy_from_slice(bytes);
    }
}

fn push_merged(out: &mut Vec<Range<u64>>, r: Range<u64>) {
    if let Some(last) = out.last_mut()
        && last.end >= r.start
    {
        last.end = last.end.max(r.end);
        return;
    }
    out.push(r);
}

impl ByteSource for SparseSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError> {
        if offset >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(self.len - offset);
        let chunks = self
            .chunks
            .lock()
            .map_err(|_| SourceError::Failed { offset, len: want })?;
        let Some((start, chunk)) = chunks.map.range(..=offset).next_back() else {
            return Err(pending(&chunks, offset, want));
        };
        let skip = usize::try_from(offset - start).unwrap_or(usize::MAX);
        let data: &[u8] = chunk;
        let Some(held) = data.get(skip..).filter(|h| !<[u8]>::is_empty(h)) else {
            return Err(pending(&chunks, offset, want));
        };
        // Short reads are allowed: the reader asks again for the rest, and a gap then answers
        // `Pending` for exactly what is missing.
        let n = held.len().min(usize::try_from(want).unwrap_or(usize::MAX));
        let (dst, src) = (buf.get_mut(..n), held.get(..n));
        if let (Some(dst), Some(src)) = (dst, src) {
            dst.copy_from_slice(src);
        }
        Ok(n)
    }
}

/// `Pending` for the gap at `offset`: up to the next chunk or `want` bytes, whichever is first.
fn pending(chunks: &Chunks, offset: u64, want: u64) -> SourceError {
    let end = chunks
        .next_start(offset)
        .map_or(offset + want, |n| n.min(offset + want));
    SourceError::Pending {
        offset,
        len: end - offset,
    }
}
