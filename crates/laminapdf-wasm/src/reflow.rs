//! Reader mode across the boundary: `WasmDocument.reflow()` returns the document's reflow as
//! JSON (`@laminapdf/sdk`'s `Reflow`), one string in one copy, as `extract` does. Only the
//! extract module has it.

use laminapdf::CancelToken;
use wasm_bindgen::prelude::*;

use crate::document::WasmDocument;
use crate::error::{JsResult, internal, lamina_error};

#[wasm_bindgen]
impl WasmDocument {
    /// The document for reader mode, as JSON: whether it can be shown and why not, its
    /// language, its blocks in the structure tree's order (figures, formulas and tables as crops
    /// with their alternate text) and the checks' measures (the extract module only).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    pub fn reflow(&self) -> JsResult<String> {
        let reflow = self
            .shared()
            .borrow()
            .reflow(&CancelToken::new())
            .map_err(|e| lamina_error(&e))?;
        serde_json::to_string(&reflow).map_err(|_| internal("the reflow could not be written as JSON"))
    }
}
