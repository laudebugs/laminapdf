//! What the page-setup dialogs read across the boundary: `WasmDocument.pageBoxes(index)`, a
//! page's boxes in user space as `setPageBoxes` takes them, and `labelRanges()`, the label
//! ranges as `setPageLabels` takes them (the extract and text modules, so the base module stays
//! in its size budget).

use laminapdf::{PageGeometry, UserSpaceRect};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// A page's boxes in user space, `[x0, y0, x1, y1]`, each as readers use it (a box the page
/// doesn't set is the one it defaults to).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageBoxesReadJs {
    /// The media box.
    pub media: [f64; 4],
    /// The crop box, cut to the media box.
    pub crop: [f64; 4],
    /// The bleed box, cut to the crop box.
    pub bleed: [f64; 4],
    /// The trim box, cut to the crop box.
    pub trim: [f64; 4],
    /// The art box, cut to the crop box.
    pub art: [f64; 4],
    /// Which of the bleed, trim and art boxes the page sets itself.
    pub own: OwnBoxesJs,
    /// The size of a user-space unit in points.
    pub user_unit: f64,
}

/// Which boxes a page sets itself, rather than taking the crop box.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct OwnBoxesJs {
    /// `/BleedBox`.
    pub bleed: bool,
    /// `/TrimBox`.
    pub trim: bool,
    /// `/ArtBox`.
    pub art: bool,
}

const fn corners(r: UserSpaceRect) -> [f64; 4] {
    [r.x0, r.y0, r.x1, r.y1]
}

impl From<PageGeometry> for PageBoxesReadJs {
    fn from(g: PageGeometry) -> Self {
        Self {
            media: corners(g.media),
            crop: corners(g.crop),
            bleed: corners(g.bleed),
            trim: corners(g.trim),
            art: corners(g.art),
            own: OwnBoxesJs {
                bleed: g.own.bleed,
                trim: g.own.trim,
                art: g.own.art,
            },
            user_unit: g.user_unit,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// Page `index`'s boxes in user space (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = pageBoxes, unchecked_return_type = "PageBoxesReadJs")]
    pub fn page_boxes(&self, index: u32) -> JsResult<JsValue> {
        let boxes = self.shared().borrow().page_boxes(index).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&PageBoxesReadJs::from(boxes)))
    }

    /// The page label ranges, in page order; empty without labels (the extract and text
    /// modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = labelRanges, unchecked_return_type = "PageLabelRange[]")]
    pub fn label_ranges(&self) -> JsResult<JsValue> {
        let ranges = self.shared().borrow().label_ranges().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&ranges))
    }
}
