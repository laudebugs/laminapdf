//! Objects added to pages as content, across the boundary: `addedObjects` lists what a page
//! holds. The SDK writes the ops that add, change and remove them itself, as JSON, so every
//! module with the builder carries only this listing.

use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

#[wasm_bindgen]
impl WasmDocument {
    /// The objects Lamina PDF added to page `page`, in the order they paint: those behind the
    /// page's content first.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    #[wasm_bindgen(js_name = addedObjects, unchecked_return_type = "AddedObject[]")]
    pub fn added_objects(&self, page: u32) -> JsResult<JsValue> {
        let objects = self
            .shared()
            .borrow()
            .added_objects(page)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&objects))
    }
}
