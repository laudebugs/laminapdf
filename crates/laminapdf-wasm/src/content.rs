//! The objects a page draws, for an editor (the extract and text modules, with the
//! viewer): `contentObjects`, `hitTest`, `hitTestRect` and `addBlob`. The `editContent` op
//! changes them through `apply`.

use laminapdf::{ContentObjectKind, ObjectRef, PageObject, PointPt, RectPt};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// One object of a page, as `contentObjects` lists it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PageObjectJs {
    object: ObjectRef,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_only: Option<&'static str>,
}

#[wasm_bindgen(typescript_custom_section)]
const PAGE_OBJECT_TS: &str = r#"
/** One object a page draws, in painting order (later ones over earlier ones). */
export type PageObjectJs = {
  /** Its name for an edit: its ordinal and the hash of its bytes. */
  object: ObjectRef;
  /** What it draws. */
  kind: "path" | "text" | "image" | "form" | "xobject" | "shading";
  /** Its box in page space, `[x0, y0, x1, y1]`, before clipping; absent when it draws nothing. */
  bounds?: [number, number, number, number];
  /** Why it can't be moved, deleted or reordered (`clipping-path`, …); absent when it can. */
  readOnly?: string;
};
"#;

/// The kind's name on the wire.
const fn kind_name(kind: ContentObjectKind) -> &'static str {
    match kind {
        ContentObjectKind::Path => "path",
        ContentObjectKind::Text => "text",
        ContentObjectKind::Image => "image",
        ContentObjectKind::Form => "form",
        ContentObjectKind::Shading => "shading",
        _ => "xobject",
    }
}

fn rect(r: &RectPt) -> [f64; 4] {
    [r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()]
}

impl From<&PageObject> for PageObjectJs {
    fn from(object: &PageObject) -> Self {
        Self {
            object: object.object,
            kind: kind_name(object.kind),
            bounds: object.bounds.as_ref().map(rect),
            read_only: object.read_only.map(laminapdf::NotEditable::as_str),
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The objects page `index` draws, in painting order (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = contentObjects, unchecked_return_type = "PageObjectJs[]")]
    pub fn content_objects(&self, index: u32) -> JsResult<JsValue> {
        let objects = self
            .shared()
            .borrow()
            .content_objects(index)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&objects.iter().map(PageObjectJs::from).collect::<Vec<_>>()))
    }

    /// The objects of page `index` whose box holds the point `(x, y)` in page space, or comes
    /// within 2 points of it, topmost first.
    ///
    /// # Errors
    /// As `contentObjects`.
    #[wasm_bindgen(js_name = hitTest, unchecked_return_type = "ObjectRef[]")]
    pub fn hit_test(&self, index: u32, x: f64, y: f64) -> JsResult<JsValue> {
        let found = self
            .shared()
            .borrow()
            .hit_test(index, PointPt::from_points(x, y))
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&found))
    }

    /// The objects of page `index` whose box meets `rect` (`[x0, y0, x1, y1]` in page space),
    /// topmost first.
    ///
    /// # Errors
    /// `invalid-argument` for a value that isn't four numbers; as `contentObjects`.
    #[wasm_bindgen(js_name = hitTestRect, unchecked_return_type = "ObjectRef[]")]
    pub fn hit_test_rect(
        &self,
        index: u32,
        #[wasm_bindgen(unchecked_param_type = "[number, number, number, number]")] rect: &JsValue,
    ) -> JsResult<JsValue> {
        let [x0, y0, x1, y1]: [f64; 4] = from_js(rect, "rect")?;
        let area = RectPt::new(
            laminapdf::Pt::new(x0),
            laminapdf::Pt::new(y0),
            laminapdf::Pt::new(x1),
            laminapdf::Pt::new(y1),
        );
        let found = self
            .shared()
            .borrow()
            .hit_test_rect(index, &area)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&found))
    }

    /// Keeps `bytes` (a replacement image's JPEG or PNG) for the session and returns their ID,
    /// 64 hex digits, which a `replaceImage` edit names them by.
    #[wasm_bindgen(js_name = addBlob)]
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> String {
        self.shared().borrow_mut().add_blob(bytes).to_string()
    }
}
