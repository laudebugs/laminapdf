//! A page's annotations across the boundary: `WasmDocument.pageAnnotations(index)`, each a record
//! in page space (the extract and text modules, as the links are, so the base module stays in
//! its size budget). The ops that change them, `updateAnnot` and `deleteAnnot`, cross through
//! `apply` in every module.

use laminapdf::{AnnotationGeometry, PageAnnotation, PointPt};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// One annotation on a page, in page space.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageAnnotationJs {
    /// Its ID (`a12.0`), as `updateAnnot` and `deleteAnnot` take it.
    pub id: String,
    /// Its position in the page's `/Annots`.
    pub index: u32,
    /// Its subtype as written (`Highlight`, `Text`, …).
    pub subtype: String,
    /// Its rectangle: `[x0, y0, x1, y1]`, top-left and bottom-right; left out when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
    /// `/Contents` (document text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    /// `/NM` (document text), only a hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `/T`, the author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// `/Subj`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// `/CreationDate` as stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// `/M` as stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// `/F`, the flags.
    pub flags: u32,
    /// `/C`: no components (transparent), one (gray), three (RGB) or four (CMYK).
    pub color: Vec<f64>,
    /// `/CA`, the opacity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// The ID of the annotation this one replies to or is grouped with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    /// Whether `inReplyTo` is a group rather than a reply.
    pub grouped: bool,
    /// The ID of its Popup.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popup: Option<String>,
    /// For a Popup, the ID of the annotation it belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Whether it has a normal appearance stream.
    pub has_appearance: bool,
    /// Whether it has rich-text contents.
    pub has_rich_text: bool,
    /// The subtype's own geometry, as flat `[x, y, …]` lists in page space.
    pub geometry: GeometryJs,
}

/// An annotation's own geometry, by `kind`; points are flat `[x, y, …]` lists in page space.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum GeometryJs {
    /// Nothing besides the rectangle.
    None,
    /// Text markup's areas, eight numbers each.
    Quads {
        /// The areas' corners.
        quads: Vec<f64>,
    },
    /// Ink strokes.
    Ink {
        /// One point list per stroke.
        strokes: Vec<Vec<f64>>,
    },
    /// A polygon's or polyline's vertices.
    Vertices {
        /// The vertices.
        points: Vec<f64>,
    },
    /// A line's two ends.
    Line {
        /// The ends.
        points: Vec<f64>,
    },
    /// A callout's points.
    Callout {
        /// Two or three points.
        points: Vec<f64>,
    },
}

fn flat<'a>(points: impl IntoIterator<Item = &'a PointPt>) -> Vec<f64> {
    points.into_iter().flat_map(|p| [p.x.get(), p.y.get()]).collect()
}

impl From<&PageAnnotation> for PageAnnotationJs {
    fn from(a: &PageAnnotation) -> Self {
        let geometry = match &a.geometry {
            AnnotationGeometry::Quads(quads) => GeometryJs::Quads {
                quads: flat(quads.iter().flatten()),
            },
            AnnotationGeometry::Ink(strokes) => GeometryJs::Ink {
                strokes: strokes.iter().map(flat).collect(),
            },
            AnnotationGeometry::Vertices(points) => GeometryJs::Vertices { points: flat(points) },
            AnnotationGeometry::Line(ends) => GeometryJs::Line { points: flat(ends) },
            AnnotationGeometry::Callout(points) => GeometryJs::Callout { points: flat(points) },
            _ => GeometryJs::None,
        };
        Self {
            id: a.id.clone(),
            index: a.index,
            subtype: a.subtype.clone(),
            rect: a.rect.map(|r| [r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()]),
            contents: a.contents.clone(),
            name: a.name.clone(),
            author: a.author.clone(),
            subject: a.subject.clone(),
            created: a.created.clone(),
            modified: a.modified.clone(),
            flags: a.flags,
            color: a.color.clone(),
            opacity: a.opacity,
            in_reply_to: a.in_reply_to.clone(),
            grouped: a.grouped,
            popup: a.popup.clone(),
            parent: a.parent.clone(),
            has_appearance: a.has_appearance,
            has_rich_text: a.has_rich_text,
            geometry,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The annotations on page `index`, in the order of its `/Annots`, every subtype included
    /// (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = pageAnnotations, unchecked_return_type = "PageAnnotationJs[]")]
    pub fn page_annotations(&self, index: u32) -> JsResult<JsValue> {
        let annots = self
            .shared()
            .borrow()
            .page_annotations(index)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&annots.iter().map(PageAnnotationJs::from).collect::<Vec<_>>()))
    }
}
