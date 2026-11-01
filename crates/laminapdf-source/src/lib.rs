//! Edge byte sources for Lamina PDF (ADR 0005 §2): inputs that are not one in-memory buffer.
//!
//! - [`SparseSource`]: a source of known length filled range by range (HTTP `Range`, a JS
//!   callback, a host file read on demand). Reads of bytes not supplied yet answer
//!   [`SourceError::Pending`](laminapdf_cos::SourceError::Pending), which the reader reports as
//!   `data-pending` with the ranges it needs.
//! - [`retry`] and [`open_with`]: the fetch-and-retry loop that turns those errors into fetches.
//! - With `native`: [`FileSource`] (positioned reads). With `mmap`: [`MmapSource`] (a mapping;
//!   read its `SIGBUS` note first).
//!
//! The sparse source and the loop are pure (no I/O, clocks or threads) and build for wasm32;
//! they live here rather than in `laminapdf-cos` because the chunk map needs a lock (ADR 0005 §7
//! keeps locks and caches out of core).

mod retry;
mod sparse;

#[cfg(feature = "native")]
mod file;
#[cfg(feature = "mmap")]
mod mmap;

pub use retry::{RetryError, RetryOptions, open_with, pending_ranges, retry};
pub use sparse::{SparseSource, SupplyError};

#[cfg(feature = "native")]
pub use file::FileSource;
#[cfg(feature = "mmap")]
pub use mmap::MmapSource;
