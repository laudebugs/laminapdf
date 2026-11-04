//! `WasmSparseSource`: a source filled range by range from JavaScript. `@laminapdf/sdk`'s
//! range adapters fetch what `data-pending` asks for and `supply` it, then retry the call.

use std::sync::Arc;

use js_sys::Float64Array;
use laminapdf_source::{SparseSource, SupplyError};
use wasm_bindgen::prelude::*;

use crate::error::{JsResult, invalid_argument, source_budget_exceeded};

/// The largest integer a JS number holds exactly.
const MAX_SAFE: f64 = 9_007_199_254_740_991.0;

/// A whole, non-negative, exactly representable number as `u64`.
fn whole(value: f64, argument: &'static str) -> JsResult<u64> {
    if (0.0..=MAX_SAFE).contains(&value) && value.fract() == 0.0 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked above: a whole number in 0..=2^53-1"
        )]
        Ok(value as u64)
    } else {
        Err(invalid_argument(
            argument,
            "a whole number of bytes from 0 to 2^53 - 1".to_owned(),
            Vec::new(),
        ))
    }
}

/// A PDF of known size whose bytes arrive later: give it to `WasmDocument.openSource`, and
/// after a `data-pending` error `supply` the ranges in its `data.ranges` and call again.
#[wasm_bindgen]
pub struct WasmSparseSource {
    pub(crate) inner: Arc<SparseSource>,
}

#[wasm_bindgen]
impl WasmSparseSource {
    /// An empty source of `size` bytes holding at most `budget` bytes (no limit when omitted).
    ///
    /// # Errors
    /// `invalid-argument` when a number isn't a whole number of bytes.
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64, budget: Option<f64>) -> JsResult<Self> {
        let size = whole(size, "size")?;
        let budget = budget.map(|b| whole(b, "budget")).transpose()?.unwrap_or(u64::MAX);
        Ok(Self {
            inner: Arc::new(SparseSource::with_budget(size, budget)),
        })
    }

    /// The source's size in bytes.
    #[wasm_bindgen(getter)]
    #[expect(clippy::cast_precision_loss, reason = "sizes come from JS numbers, so they fit")]
    pub fn size(&self) -> f64 {
        laminapdf::ByteSource::len(self.inner.as_ref()) as f64
    }

    /// Adds `bytes` at `offset` (bytes already held win over overlapping ones).
    ///
    /// # Errors
    /// `invalid-argument` past the end; `source-budget-exceeded` over the budget.
    pub fn supply(&self, offset: f64, bytes: &[u8]) -> JsResult<()> {
        let offset = whole(offset, "offset")?;
        self.inner.supply(offset, bytes).map_err(supply_error)
    }

    /// What to fetch for the ranges a `data-pending` asked for (flat `[start, end, …]`): each
    /// widened to blocks of `block` bytes (0: as asked) plus `readahead`, merged, minus what is
    /// held, in the same flat form. The planning step of `laminapdf-source`'s `retry`, so every
    /// loop plans the same way.
    ///
    /// # Errors
    /// `invalid-argument` for an odd length or a value that isn't a whole number of bytes.
    #[wasm_bindgen(js_name = toFetch)]
    pub fn to_fetch(&self, asked: &[f64], block: f64, readahead: f64) -> JsResult<Float64Array> {
        let asked = pairs(asked)?;
        let ranges = self
            .inner
            .to_fetch(&asked, whole(block, "block")?, whole(readahead, "readahead")?)
            .map_err(supply_error)?;
        Ok(flat(&ranges))
    }
}

/// Flat `[start, end, …]` as ranges.
fn pairs(flat: &[f64]) -> JsResult<Vec<std::ops::Range<u64>>> {
    if !flat.len().is_multiple_of(2) {
        return Err(invalid_argument(
            "ranges",
            "pairs of start and end".to_owned(),
            Vec::new(),
        ));
    }
    flat.as_chunks::<2>()
        .0
        .iter()
        .map(|[start, end]| Ok(whole(*start, "ranges")?..whole(*end, "ranges")?))
        .collect()
}

/// Ranges as flat `[start, end, …]`.
fn flat(ranges: &[std::ops::Range<u64>]) -> Float64Array {
    #[expect(clippy::cast_precision_loss, reason = "offsets are within the size, which fits")]
    let flat: Vec<f64> = ranges.iter().flat_map(|r| [r.start as f64, r.end as f64]).collect();
    Float64Array::from(flat.as_slice())
}

/// A supply error as the §1 object: over the budget is `source-budget-exceeded`; a chunk past
/// the end is `invalid-argument` on `bytes` (the count is what's wrong: the source has a fixed
/// size). A poisoned lock can't happen in WASM (`panic = "abort"`).
fn supply_error(e: SupplyError) -> JsValue {
    match e {
        SupplyError::Budget { needed, budget } => {
            source_budget_exceeded(format!("would hold {needed} bytes; the budget is {budget}"))
        }
        other => invalid_argument("bytes", other.to_string(), Vec::new()),
    }
}
