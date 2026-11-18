//! Text boxes and callouts across the boundary: the edits that put a `FreeText` annotation on a
//! page, from a box and points in page space. Every module with the builder makes them; laying
//! out the text needs the extract module (Helvetica, Times and Courier) or the text module (any
//! font), and the base module refuses the edit with `appearance-needs-text` when it is applied.
//! `textBox(id)` reads a box back as the spec it holds (the extract and text modules, as
//! `pageAnnotations` is).

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

    /// The text box `id` read back, whichever app made it: where it is in page space, the spec
    /// a `freeText` patch changes, its colour, and whether reading it dropped styles Lamina PDF
    /// doesn't keep. `undefined` for an annotation of another kind (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't an annotation's; `unknown-annotation`;
    /// `annotation-not-editable`; `data-pending` and the reading errors.
    #[cfg(feature = "extract")]
    #[wasm_bindgen(js_name = textBox, unchecked_return_type = "TextBoxJs | undefined")]
    pub fn text_box(&self, id: &str) -> JsResult<JsValue> {
        let id: laminapdf::AnnotId = id
            .parse()
            .map_err(|_| crate::error::invalid_argument("id", "not an annotation ID".to_owned(), Vec::new()))?;
        let read = self.shared().borrow().text_box(&id).map_err(|e| lamina_error(&e))?;
        Ok(read.as_ref().map_or(JsValue::UNDEFINED, |b| to_js(&TextBoxJs::from(b))))
    }
}

/// A text box read back, in page space beside its spec in user space.
#[cfg(feature = "extract")]
#[derive(Debug, serde::Serialize, tsify::Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TextBoxJs {
    /// Its page, 0-based.
    pub page: u32,
    /// The box the text is laid out in: `[x0, y0, x1, y1]`, top-left and bottom-right.
    pub rect: [f64; 4],
    /// A callout's line, `[x, y]` points from the point it points at to the box; empty for
    /// another kind.
    pub line: Vec<[f64; 2]>,
    /// The box as a spec, in user space: what a `freeText` patch changes.
    pub spec: laminapdf::FreeText,
    /// The text's colour; black when left out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<laminapdf::AnnotColor>,
    /// Whether reading dropped styles or a font the spec can't hold; changing the box writes
    /// it again without them (`rich-text-simplified`).
    pub simplified: bool,
}

#[cfg(feature = "extract")]
impl From<&laminapdf::TextBox> for TextBoxJs {
    fn from(b: &laminapdf::TextBox) -> Self {
        Self {
            page: b.page,
            rect: [b.rect.x0.get(), b.rect.y0.get(), b.rect.x1.get(), b.rect.y1.get()],
            line: b.line.iter().map(|p| [p.x.get(), p.y.get()]).collect(),
            spec: b.spec.clone(),
            color: b.color,
            simplified: b.simplified,
        }
    }
}

/// A string as plain text, else rich text.
fn rich_text(text: &JsValue) -> JsResult<RichText> {
    match text.as_string() {
        Some(plain) => Ok(RichText::plain(&plain)),
        None => from_js(text, "text"),
    }
}
