//! Marking up a page across the boundary: the edits that highlight, underline, strike out or
//! squiggle text, put a note or a caret on a page, propose a text replacement, and draw ink,
//! shapes, lines and polygons. Every module with the builder has them (it draws the
//! appearances); in the base module, which can't read text, a text mark has no marked content or
//! default description.

use laminapdf::{AnnotCommon, LineEnding, MarkupKind, NoteIcon, PointPt, PolyKind, Pt, RectPt, ShapeKind};
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, lamina_error};

#[wasm_bindgen]
impl WasmDocument {
    /// The edit that marks text of page `page` with `kind`: `lines` are the text's boxes, one per
    /// line, each `[x0, y0, x1, y1]` in page space, as a selection gives them. Its arguments are
    /// an `addAnnot` op's.
    ///
    /// # Errors
    /// `invalid-argument` for a kind or lines that aren't ones; `invalid-op` for no lines;
    /// `page-index-out-of-range`; the page's reading errors.
    #[wasm_bindgen(js_name = markText, unchecked_return_type = "AddAnnot")]
    pub fn mark_text(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "MarkupKind")] kind: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number][]")] lines: &JsValue,
    ) -> JsResult<JsValue> {
        let kind: MarkupKind = from_js(kind, "kind")?;
        let lines = boxes(lines)?;
        let op = self
            .shared()
            .borrow()
            .mark_text(page, kind, &lines)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that puts a note on page `page`: `icon` with its upper-left corner at `at`
    /// (`[x, y]` in page space), and `comment` in its Popup.
    ///
    /// # Errors
    /// `invalid-argument` for a point or icon that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = noteAt, unchecked_return_type = "AddAnnot")]
    pub fn note_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number]")] at: &JsValue,
        comment: &str,
        #[wasm_bindgen(unchecked_param_type = "NoteIcon | undefined")] icon: &JsValue,
    ) -> JsResult<JsValue> {
        let [x, y]: [f64; 2] = from_js(at, "at")?;
        let icon: NoteIcon = from_js_or_default(icon, "icon")?;
        let op = self
            .shared()
            .borrow()
            .note_at(page, PointPt::from_points(x, y), comment, icon)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that puts a caret on page `page`, filling `area` (`[x0, y0, x1, y1]` in page
    /// space).
    ///
    /// # Errors
    /// `invalid-argument` for an area that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = caretAt, unchecked_return_type = "AddAnnot")]
    pub fn caret_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] area: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0, x1, y1]: [f64; 4] = from_js(area, "area")?;
        let op = self
            .shared()
            .borrow()
            .caret_at(page, rect(x0, y0, x1, y1))
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// Proposes replacing the text in `lines` of page `page` with `replacement`: a strike-out
    /// and a caret, grouped, applied as one undo step. `common` (optional) goes to both.
    ///
    /// # Errors
    /// `invalid-argument` for lines or common entries that aren't ones; as `markText`; the
    /// edits' errors.
    #[wasm_bindgen(js_name = replaceText, unchecked_return_type = "EditedJs")]
    pub fn replace_text(
        &mut self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number][]")] lines: &JsValue,
        replacement: &str,
        #[wasm_bindgen(unchecked_param_type = "AnnotCommon | undefined")] common: &JsValue,
    ) -> JsResult<JsValue> {
        let lines = boxes(lines)?;
        let common: AnnotCommon = from_js_or_default(common, "common")?;
        let edited = self
            .shared()
            .borrow_mut()
            .replace_text(page, &lines, replacement, common)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }

    /// The edit that draws freehand ink on page `page`: one stroke per list of `[x, y]` points
    /// in page space, by a pen `width` points wide, smoothed. Its arguments are an `addAnnot`
    /// op's; give a stroke `widths`, one per point, for a pressure-sensitive pen.
    ///
    /// # Errors
    /// `invalid-argument` for strokes that aren't lists of points; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = inkAt, unchecked_return_type = "AddAnnot")]
    pub fn ink_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number][][]")] strokes: &JsValue,
        width: f64,
    ) -> JsResult<JsValue> {
        let strokes: Vec<Vec<[f64; 2]>> = from_js(strokes, "strokes")?;
        let strokes: Vec<Vec<PointPt>> = strokes
            .iter()
            .map(|s| s.iter().map(|&[x, y]| PointPt::from_points(x, y)).collect())
            .collect();
        let op = self
            .shared()
            .borrow()
            .ink_at(page, &strokes, width)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that draws a square or circle on page `page` along `area` (`[x0, y0, x1, y1]`
    /// in page space).
    ///
    /// # Errors
    /// `invalid-argument` for a kind or area that isn't one; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = shapeAt, unchecked_return_type = "AddAnnot")]
    pub fn shape_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "ShapeKind")] kind: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] area: &JsValue,
    ) -> JsResult<JsValue> {
        let kind: ShapeKind = from_js(kind, "kind")?;
        let [x0, y0, x1, y1]: [f64; 4] = from_js(area, "area")?;
        let op = self
            .shared()
            .borrow()
            .shape_at(page, kind, rect(x0, y0, x1, y1))
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that draws a line on page `page` from `from` to `to` (`[x, y]` in page space),
    /// with `ends` (`[start, end]`, none by default) at its ends.
    ///
    /// # Errors
    /// `invalid-argument` for points or endings that aren't ones; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = lineAt, unchecked_return_type = "AddAnnot")]
    pub fn line_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number]")] from: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[number, number]")] to: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[LineEnding, LineEnding] | undefined")] ends: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0]: [f64; 2] = from_js(from, "from")?;
        let [x1, y1]: [f64; 2] = from_js(to, "to")?;
        let ends: [LineEnding; 2] = from_js_or_default(ends, "ends")?;
        let op = self
            .shared()
            .borrow()
            .line_at(page, PointPt::from_points(x0, y0), PointPt::from_points(x1, y1), ends)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }

    /// The edit that draws a polygon or polyline on page `page` through `vertices` (`[x, y]`
    /// in page space).
    ///
    /// # Errors
    /// `invalid-argument` for a kind or vertices that aren't ones; `page-index-out-of-range`.
    #[wasm_bindgen(js_name = polyAt, unchecked_return_type = "AddAnnot")]
    pub fn poly_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "PolyKind")] kind: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "[number, number][]")] vertices: &JsValue,
    ) -> JsResult<JsValue> {
        let kind: PolyKind = from_js(kind, "kind")?;
        let vertices: Vec<[f64; 2]> = from_js(vertices, "vertices")?;
        let vertices: Vec<PointPt> = vertices.iter().map(|&[x, y]| PointPt::from_points(x, y)).collect();
        let op = self
            .shared()
            .borrow()
            .poly_at(page, kind, &vertices)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&op))
    }
}

/// Line boxes, each `[x0, y0, x1, y1]` in page space.
fn boxes(lines: &JsValue) -> JsResult<Vec<RectPt>> {
    let boxes: Vec<[f64; 4]> = from_js(lines, "lines")?;
    Ok(boxes.iter().map(|&[x0, y0, x1, y1]| rect(x0, y0, x1, y1)).collect())
}

/// A page-space rectangle from its corners.
pub(crate) fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
