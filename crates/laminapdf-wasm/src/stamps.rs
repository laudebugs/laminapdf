//! Stamps and files attached to a spot on a page, across the boundary: the edits that put a
//! `Stamp` or a `FileAttachment` annotation on a page, from a box or a point in page space. Every
//! module with the builder makes them and draws standard and image stamps; a dynamic stamp's
//! lines need the extract or text module, and the base module refuses it with
//! `appearance-needs-text` when it is applied.

use laminapdf::{PointPt, StampKind};
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};
use crate::marks::rect;

#[wasm_bindgen]
impl WasmDocument {
    /// The edit that stamps page `page` over `area` (`[x0, y0, x1, y1]` in page space) with
    /// `stamp`: `{ standard: "approved" }`, `{ image: blobId }` or `{ dynamic: { lines, style }
    /// }`, upright as the page is shown.
    ///
    /// # Errors
    /// `invalid-argument` for an area or stamp that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = stampAt, unchecked_return_type = "AddAnnot")]
    pub fn stamp_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] area: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "StampKind")] stamp: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0, x1, y1]: [f64; 4] = from_js(area, "area")?;
        let stamp: StampKind = from_js(stamp, "stamp")?;
        let op = self
            .shared()
            .borrow()
            .stamp_at(page, rect(x0, y0, x1, y1), stamp)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that attaches `bytes` as the file `name` of type `mime` (empty for none) at `at`
    /// (`[x, y]` in page space, the icon's upper-left corner) on page `page`. The bytes are kept
    /// for the session, as `addBlob` keeps them.
    ///
    /// # Errors
    /// `invalid-argument` for a point that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = attachFileAt, unchecked_return_type = "AddAnnot")]
    pub fn attach_file_at(
        &mut self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number]")] at: &JsValue,
        name: &str,
        bytes: Vec<u8>,
        mime: &str,
    ) -> JsResult<JsValue> {
        let [x, y]: [f64; 2] = from_js(at, "at")?;
        let op = self
            .shared()
            .borrow_mut()
            .attach_file_at(page, PointPt::from_points(x, y), name, bytes, mime)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }
}
