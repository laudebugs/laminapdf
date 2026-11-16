//! Text boxes and callouts across the boundary: the edits that put a `FreeText` annotation on a
//! page, from a box and points in page space. Every module with the builder makes them; laying
//! out the text needs the extract module (Helvetica, Times and Courier) or the text module (any
//! font), and the base module refuses the edit with `appearance-needs-text` when it is applied.

use laminapdf::{PointPt, RichText};
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};
use crate::marks::rect;

#[wasm_bindgen]
impl WasmDocument {
    /// The edit that adds a text box on page `page` over `area` (`[x0, y0, x1, y1]` in page
    /// space) holding `text`: a string, a paragraph per line, or rich text (an array of
    /// paragraphs, each an array of `{ text, bold?, italic? }` spans).
    ///
    /// # Errors
    /// `invalid-argument` for an area or text that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = textBoxAt, unchecked_return_type = "AddAnnot")]
    pub fn text_box_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] area: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "string | RichText")] text: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0, x1, y1]: [f64; 4] = from_js(area, "area")?;
        let op = self
            .shared()
            .borrow()
            .text_box_at(page, rect(x0, y0, x1, y1), rich_text(text)?)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that adds a callout on page `page`: a text box over `area` with a leader line
    /// through `line` (`[x, y]` points in page space, from the point it points at, through an
    /// optional knee, to the box) ending in an open arrow.
    ///
    /// # Errors
    /// `invalid-argument` for an area, line or text that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = calloutAt, unchecked_return_type = "AddAnnot")]
    pub fn callout_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] area: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[number, number][]")] line: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "string | RichText")] text: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0, x1, y1]: [f64; 4] = from_js(area, "area")?;
        let line: Vec<[f64; 2]> = from_js(line, "line")?;
        let line: Vec<PointPt> = line.iter().map(|&[x, y]| PointPt::from_points(x, y)).collect();
        let op = self
            .shared()
            .borrow()
            .callout_at(page, rect(x0, y0, x1, y1), &line, rich_text(text)?)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }
}

/// A string as plain text, else rich text.
fn rich_text(text: &JsValue) -> JsResult<RichText> {
    match text.as_string() {
        Some(plain) => Ok(RichText::plain(&plain)),
        None => from_js(text, "text"),
    }
}
