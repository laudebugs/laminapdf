//! The fetch-and-retry loop over `data-pending`: [`retry`] and [`open_with`].

use std::ops::Range;
use std::sync::Arc;

use lamina::{ByteSource, Document, OpenOptions};

use crate::sparse::{SparseSource, SupplyError};

/// How [`retry`] plans its fetches.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryOptions {
    /// Rounds (fetch, then call again) before giving up with the last `data-pending` (default
    /// 1024). A backstop: a round that asks again for what it was just given stops the loop
    /// first. Reading one large stream can take a few rounds (the reader asks for the next gap
    /// as it meets it), so a full-file walk of a big document takes many.
    pub max_rounds: u32,
    /// Fetches are widened to whole blocks of this many bytes (default 4 KiB; 0: exactly what
    /// was asked). Small on purpose: the reader checks each object's xref entry with a short
    /// probe, and 64 KiB blocks around every probe would fetch most of the file.
    pub block_size: u64,
    /// Extra bytes fetched after each range, for the next read that is likely to follow.
    pub readahead: u64,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_rounds: 1024,
            block_size: 4096,
            readahead: 0,
        }
    }
}

impl RetryOptions {
    /// Sets [`RetryOptions::max_rounds`].
    #[must_use]
    pub fn with_max_rounds(mut self, rounds: u32) -> Self {
        self.max_rounds = rounds;
        self
    }

    /// Sets [`RetryOptions::block_size`].
    #[must_use]
    pub fn with_block_size(mut self, bytes: u64) -> Self {
        self.block_size = bytes;
        self
    }

    /// Sets [`RetryOptions::readahead`].
    #[must_use]
    pub fn with_readahead(mut self, bytes: u64) -> Self {
        self.readahead = bytes;
        self
    }
}

/// Why [`retry`] or [`open_with`] failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RetryError<E> {
    /// The call failed. A `data-pending` here means the loop gave up: the rounds ran out, or a
    /// round asked again for ranges it had just been given; its ranges are the last ones asked.
    #[error(transparent)]
    Lamina(lamina::Error),
    /// The shell's fetch failed.
    #[error("fetching input bytes failed")]
    Fetch(#[source] E),
    /// A fetched chunk couldn't be supplied (past the end, or over the source's budget).
    #[error(transparent)]
    Supply(SupplyError),
}

/// The ranges a `data-pending` error asks for, if it is one.
pub fn pending_ranges(error: &lamina::Error) -> Option<Vec<Range<u64>>> {
    match error {
        lamina::Error::Cos(lamina::CosError::DataPending { ranges }) => Some(
            ranges
                .iter()
                .map(|&(offset, len)| offset..offset.saturating_add(len))
                .collect(),
        ),
        _ => None,
    }
}

/// Runs `call` until it stops answering `data-pending`: each time, the ranges it asks for are
/// widened by `options`, reduced to what `source` doesn't hold yet, fetched with `fetch` (one
/// call per range, in file order) and supplied. Synchronous, so it suits native shells and the
/// wasm edge alike; async shells (TypeScript) run the same loop with `await`.
///
/// # Errors
/// [`RetryError::Lamina`] for the call's own errors, and its last `data-pending` when
/// `max_rounds` run out or a round makes no progress (nothing left to fetch, or the same
/// ranges as the round before); [`RetryError::Fetch`] and [`RetryError::Supply`] from fetching.
pub fn retry<T, E>(
    source: &SparseSource,
    options: &RetryOptions,
    mut fetch: impl FnMut(Range<u64>) -> Result<Vec<u8>, E>,
    mut call: impl FnMut() -> lamina::Result<T>,
) -> Result<T, RetryError<E>> {
    let mut rounds = 0_u32;
    let mut previous: Option<Vec<Range<u64>>> = None;
    loop {
        let error = match call() {
            Ok(value) => return Ok(value),
            Err(e) => e,
        };
        let Some(asked) = pending_ranges(&error) else {
            return Err(RetryError::Lamina(error));
        };
        let planned = plan(&asked, options, source.len());
        let todo = source.missing(&planned).map_err(RetryError::Supply)?;
        if rounds >= options.max_rounds || todo.is_empty() || previous.as_ref() == Some(&asked) {
            return Err(RetryError::Lamina(error));
        }
        for range in todo {
            let offset = range.start;
            let bytes = fetch(range).map_err(RetryError::Fetch)?;
            source.supply(offset, &bytes).map_err(RetryError::Supply)?;
        }
        previous = Some(asked);
        rounds += 1;
    }
}

/// Opens a document over `source`, fetching what the open needs (see [`retry`]). Later lazy
/// calls on the document can still answer `data-pending`; wrap them in [`retry`] with the same
/// source.
///
/// # Errors
/// As [`retry`].
pub fn open_with<E>(
    source: &Arc<SparseSource>,
    open: &OpenOptions,
    options: &RetryOptions,
    fetch: impl FnMut(Range<u64>) -> Result<Vec<u8>, E>,
) -> Result<Document, RetryError<E>> {
    retry(source, options, fetch, || {
        let shared: Arc<dyn ByteSource> = Arc::clone(source) as Arc<dyn ByteSource>;
        Document::open_source(shared, open.clone())
    })
}

/// Widens `ranges` to whole blocks plus readahead, clamped to `len`, sorted and merged.
fn plan(ranges: &[Range<u64>], options: &RetryOptions, len: u64) -> Vec<Range<u64>> {
    let mut out: Vec<Range<u64>> = ranges
        .iter()
        .map(|r| {
            let end = r.end.saturating_add(options.readahead);
            match options.block_size {
                0 => r.start..end,
                b => (r.start / b * b)..end.div_ceil(b).saturating_mul(b),
            }
        })
        .map(|r| r.start.min(len)..r.end.min(len))
        .filter(|r| r.start < r.end)
        .collect();
    out.sort_by_key(|r| r.start);
    let mut merged: Vec<Range<u64>> = Vec::with_capacity(out.len());
    for r in out {
        match merged.last_mut() {
            Some(last) if last.end >= r.start => last.end = last.end.max(r.end),
            _ => merged.push(r),
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_aligns_to_blocks_adds_readahead_and_merges() {
        let o = RetryOptions::default().with_block_size(10).with_readahead(5);
        assert_eq!(plan(&[3..4, 12..13, 40..41], &o, 45), vec![0..20, 40..45]);
        let exact = RetryOptions::default().with_block_size(0);
        assert_eq!(plan(&[3..4, 9..12], &exact, 100), vec![3..4, 9..12]);
    }
}
