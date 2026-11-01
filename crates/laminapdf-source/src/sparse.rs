//! [`SparseSource`]: a [`ByteSource`] that starts empty and is filled range by range.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::{Arc, Mutex, MutexGuard};

use laminapdf_cos::{ByteSource, SourceError};

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
/// Chunks are `Arc<[u8]>` that never overlap (a supply stores only what it adds, so it copies
/// no more than it is given), and their total is bounded by a byte budget.
/// Supplied bytes are never evicted: the document may already have read them. No I/O, clocks
/// or threads; the chunk map is behind a lock because `supply` runs while the document shares
/// the source, which is why this lives in an edge crate (ADR 0005 §2, §7).
#[derive(Debug)]
pub struct SparseSource {
    len: u64,
    budget: u64,
    chunks: Mutex<Chunks>,
}

/// Supplied chunks by first byte; never overlapping (each supply stores only the gaps it fills).
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

    /// Adds `bytes` at `offset`. Only the parts not held yet are stored (bytes already held win
    /// over overlapping new ones: the document may have read them), each as its own chunk, so a
    /// supply copies at most `bytes.len()` bytes whatever the source already holds.
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
        let mut chunks = self.lock()?;
        let gaps = chunks.gaps(offset..end);
        let added: u64 = gaps.iter().map(|g| g.end - g.start).sum();
        let needed = chunks.held.saturating_add(added);
        if needed > self.budget {
            return Err(SupplyError::Budget {
                needed,
                budget: self.budget,
            });
        }
        for gap in gaps {
            let from = usize::try_from(gap.start - offset).unwrap_or(usize::MAX);
            let to = usize::try_from(gap.end - offset).unwrap_or(usize::MAX);
            if let Some(piece) = bytes.get(from..to) {
                chunks.map.insert(gap.start, Arc::from(piece));
            }
        }
        chunks.held = needed;
        Ok(())
    }

    /// What to fetch for the ranges a `data-pending` asked for: each widened to whole blocks of
    /// `block` bytes (0: as asked) plus `readahead`, clamped to the source, sorted and merged,
    /// minus what is held. The planning step of [`retry`](crate::retry), shared with the
    /// bindings that run the loop themselves.
    ///
    /// # Errors
    /// [`SupplyError::Poisoned`] after a panic while the map was locked.
    pub fn to_fetch(&self, asked: &[Range<u64>], block: u64, readahead: u64) -> Result<Vec<Range<u64>>, SupplyError> {
        let widened: Vec<Range<u64>> = asked
            .iter()
            .map(|r| {
                let end = r.end.saturating_add(readahead);
                match block {
                    0 => r.start..end,
                    b => (r.start / b * b)..end.div_ceil(b).saturating_mul(b),
                }
            })
            .collect();
        self.missing(&widened)
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
            for gap in chunks.gaps(r) {
                push_merged(&mut out, gap);
            }
        }
        Ok(out)
    }

    fn lock(&self) -> Result<MutexGuard<'_, Chunks>, SupplyError> {
        self.chunks.lock().map_err(|_| SupplyError::Poisoned)
    }
}

impl Chunks {
    /// The parts of `range` no chunk holds, in order.
    fn gaps(&self, range: Range<u64>) -> Vec<Range<u64>> {
        let mut out = Vec::new();
        let mut at = range.start;
        while at < range.end {
            if let Some(held_to) = self.covering(at) {
                at = held_to.min(range.end);
            } else {
                let gap_end = self.next_start(at).map_or(range.end, |n| n.min(range.end));
                out.push(at..gap_end);
                at = gap_end;
            }
        }
        out
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_fetch_aligns_to_blocks_adds_readahead_and_merges() {
        let s = SparseSource::new(45);
        assert_eq!(s.to_fetch(&[3..4, 12..13, 40..41], 10, 5), Ok(vec![0..20, 40..45]));
        assert_eq!(s.to_fetch(&[3..4, 9..12], 0, 0), Ok(vec![3..4, 9..12]));
    }

    #[test]
    fn to_fetch_leaves_out_what_is_held() {
        let s = SparseSource::new(100);
        s.supply(10, &[0; 10]).unwrap_or_default();
        assert_eq!(s.to_fetch(&[12..13, 18..21], 8, 0), Ok(vec![8..10, 20..24]));
    }
}
