//! A page's annotations across the boundary: `WasmDocument.pageAnnotations(index)`, each a record
//! in page space, and the review's threads, `WasmDocument.commentThreads(pages)` (the extract and
//! text modules, as the links are, so the base module stays in its size budget), with
//! `deleteAnnotations(ids)` and `flattenAnnotations(scope, intent)`. The ops that change them, `updateAnnot`, `deleteAnnot`, `addReply`
//! and `setReviewState`, cross through `apply` in every module.

use laminapdf::{
    AnnotId, AnnotationGeometry, BrokenThread, CommentPart, FlattenIntent, FlattenScope, PageAnnotation, PageComment,
    PointPt, StateModel,
};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, lamina_error};

/// Which annotations `flattenAnnotations` flattens: `"all"`, `{ pages: [0, 2] }` (0-based) or
/// `{ annotations: ["a12.0"] }`.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum FlattenScopeInit {
    /// Every comment and mark on every page.
    All,
    /// Every comment and mark on these pages.
    Pages(Vec<u32>),
    /// These annotations, in this order.
    Annotations(Vec<String>),
}

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
    /// For a review state set on the annotation it replies to, that state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<StateJs>,
}

/// A review state: its model (`Review`, `Marked`, or another a writer chose) and its name.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct StateJs {
    /// `Review`, `Marked`, or the model as written.
    pub model: String,
    /// The state as written (`Accepted`, `Marked`, …).
    pub state: String,
}

/// A comment and what answers it.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommentJs {
    /// The page the comment is on, from 0.
    pub page: u32,
    /// The comment.
    pub comment: PageAnnotationJs,
    /// The replies, in document order.
    pub replies: Vec<CommentPartJs>,
    /// The annotations grouped with it.
    pub group: Vec<CommentPartJs>,
    /// The review states set in the thread, in document order.
    pub states: Vec<CommentPartJs>,
    /// The latest `Review` status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,
    /// The latest `Marked` state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marked: Option<String>,
    /// For text markup, the text under it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quoted: Option<String>,
    /// `"missing-parent"` or `"cycle"` when the thread doesn't start at a comment that answers
    /// nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "\"missing-parent\" | \"cycle\"")]
    pub broken: Option<&'static str>,
}

/// An annotation of a thread, with its page.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct CommentPartJs {
    /// Its page, from 0.
    pub page: u32,
    /// The annotation.
    pub annotation: PageAnnotationJs,
}

impl From<&CommentPart> for CommentPartJs {
    fn from(p: &CommentPart) -> Self {
        Self {
            page: p.page,
            annotation: PageAnnotationJs::from(&p.annotation),
        }
    }
}

impl From<&PageComment> for CommentJs {
    fn from(c: &PageComment) -> Self {
        let parts = |list: &[CommentPart]| list.iter().map(CommentPartJs::from).collect();
        Self {
            page: c.page,
            comment: PageAnnotationJs::from(&c.comment),
            replies: parts(&c.replies),
            group: parts(&c.group),
            states: parts(&c.states),
            review: c.review.clone(),
            marked: c.marked.clone(),
            quoted: c.quoted.clone(),
            broken: c.broken.map(BrokenThread::name),
        }
    }
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
            state: a.state.as_ref().map(|s| StateJs {
                model: match &s.model {
                    StateModel::Review => "Review".to_owned(),
                    StateModel::Marked => "Marked".to_owned(),
                    StateModel::Other(other) => other.clone(),
                    _ => String::new(),
                },
                state: s.state.clone(),
            }),
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

    /// The comments on `pages` (every page when `undefined`), as threads in document order
    /// (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for an index that isn't a whole number from 0;
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = commentThreads, unchecked_return_type = "CommentJs[]")]
    pub fn comment_threads(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number[] | undefined")] pages: &JsValue,
    ) -> JsResult<JsValue> {
        // Read through serde, which refuses a negative or fractional index (`invalid-argument`).
        let pages: Option<Vec<u32>> = if pages.is_undefined() {
            None
        } else {
            Some(from_js(pages, "pages")?)
        };
        let threads = self
            .shared()
            .borrow()
            .comment_threads(pages.as_deref())
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&threads.iter().map(CommentJs::from).collect::<Vec<_>>()))
    }

    /// Deletes the annotations `ids` (`a12.0`) in one transaction, leaving out those another's
    /// deletion takes (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for a value that isn't an annotation ID; `unknown-annotation`,
    /// `annotation-not-editable`, `annotation-locked`.
    #[wasm_bindgen(js_name = deleteAnnotations, unchecked_return_type = "EditedJs")]
    pub fn delete_annotations(
        &self,
        #[wasm_bindgen(unchecked_param_type = "string[]")] ids: &JsValue,
    ) -> JsResult<JsValue> {
        let ids: Vec<String> = from_js(ids, "ids")?;
        let ids: Vec<AnnotId> = ids.iter().map(|id| id.parse()).collect::<Result<_, _>>().map_err(|_| {
            crate::error::invalid_argument("ids", "an annotation ID such as a12.0".to_owned(), Vec::new())
        })?;
        let edited = self
            .shared()
            .borrow_mut()
            .delete_annotations(&ids)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }

    /// Flattens the annotations of `scope` for `intent` (`"view"` or `"print"`) in one
    /// transaction: each appearance drawn into its page and the annotation removed (the extract
    /// and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for a scope or intent of the wrong shape, or a value that isn't an
    /// annotation ID; `page-index-out-of-range`, `unknown-annotation`, `flatten-widget`,
    /// `annotation-locked`, `annotation-not-editable`.
    #[wasm_bindgen(js_name = flattenAnnotations, unchecked_return_type = "EditedJs")]
    pub fn flatten_annotations(
        &self,
        #[wasm_bindgen(unchecked_param_type = "FlattenScopeInit")] scope: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "FlattenIntent")] intent: &JsValue,
    ) -> JsResult<JsValue> {
        let scope = match from_js::<FlattenScopeInit>(scope, "scope")? {
            FlattenScopeInit::All => FlattenScope::All,
            FlattenScopeInit::Pages(pages) => FlattenScope::Pages(pages),
            FlattenScopeInit::Annotations(ids) => {
                FlattenScope::Annotations(ids.iter().map(|id| id.parse()).collect::<Result<_, _>>().map_err(|_| {
                    crate::error::invalid_argument("scope", "an annotation ID such as a12.0".to_owned(), Vec::new())
                })?)
            }
        };
        let intent: FlattenIntent = from_js(intent, "intent")?;
        let edited = self
            .shared()
            .borrow_mut()
            .flatten_annotations(&scope, intent)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }
}
