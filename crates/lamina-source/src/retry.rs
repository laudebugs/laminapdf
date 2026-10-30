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
    /// The fetch returned a different number of bytes than the range it was asked for.
    #[error("a fetch of {expected} bytes at {offset} returned {got}")]
    FetchLength {
        /// First byte of the range.
        offset: u64,
        /// The range's length.
        expected: u64,
        /// Bytes returned.
        got: u64,
    },
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
/// ranges as the round before); [`RetryError::Fetch`], [`RetryError::FetchLength`] and
/// [`RetryError::Supply`] from fetching.
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
        let todo = source
            .to_fetch(&asked, options.block_size, options.readahead)
            .map_err(RetryError::Supply)?;
        if rounds >= options.max_rounds || todo.is_empty() || previous.as_ref() == Some(&asked) {
            return Err(RetryError::Lamina(error));
        }
        for range in todo {
            let (offset, expected) = (range.start, range.end - range.start);
            let bytes = fetch(range).map_err(RetryError::Fetch)?;
            if bytes.len() as u64 != expected {
                return Err(RetryError::FetchLength {
                    offset,
                    expected,
                    got: bytes.len() as u64,
                });
            }
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
