//! `SetAlt`: the alternative text of the figure a page's image belongs to, so a reader who
//! selected the image can describe it for a screen reader (ISO 32000-2 §14.9.3).

use std::collections::BTreeSet;

use laminapdf_content::{ContentObjectId, ContentOps, ContentOwner, interpret_page};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_syntax::PdfString;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};
use serde::{Deserialize, Serialize};

use super::structure::{self, PageTags};
use super::{ObjectRef, find};
use crate::annot::object_dict;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::pages::edit_dict;

/// The longest alternative text, in characters.
const MAX_ALT_CHARS: usize = 2000;

/// Sets the alternative text (`/Alt`) of the `Figure` structure element that owns an object the
/// page draws: the element of the object's marked content, or the nearest `Figure` above it.
/// Text the figure had is replaced. The text can't be empty and holds at most 2,000 characters.
///
/// The file must be tagged, and `object` must be drawn inside marked content that one `Figure`
/// owns: an untagged file or page, an object outside every marked-content sequence, an object
/// no `Figure` owns, an object whose marked content belongs to more than one `Figure`, and a
/// form that is tagged itself (its own `/StructParents`: the figures inside it aren't the page's)
/// are refused with `invalid-op`. The page's content isn't changed. A name whose hash no longer
/// matches fails with `content-object-stale`, once the file and page are known to be tagged.
///
/// A figure may own several objects (a chart drawn as many paths): the text is the figure's, so
/// setting it through one of them changes it for all of them.
///
/// ```
/// use laminapdf_model::PageId;
/// use laminapdf_ops::{ObjectRef, Op, SetAlt};
///
/// let page: PageId = "p3.0".parse()?;
/// let op = Op::from(SetAlt::new(page, ObjectRef::new(0, 0xa3f1), "A grey square"));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"op":"setAlt","args":{"page":"p3.0","object":{"ordinal":0,"hash":"000000000000a3f1"},"alt":"A grey square"}}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetAlt {
    /// The page.
    #[serde(with = "crate::wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The object, as `laminapdf::Document::content_objects` names it.
    pub object: ObjectRef,
    /// The alternative text.
    pub alt: String,
}

impl SetAlt {
    /// The op that sets the figure of `object` on `page` to describe it as `alt`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(page: PageId, object: ObjectRef, alt: impl Into<String>) -> Self {
        Self {
            page,
            object,
            alt: alt.into(),
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        if self.alt.trim().is_empty() {
            return Err(Error::InvalidOp("setAlt needs alternative text that isn't empty"));
        }
        if self.alt.chars().count() > MAX_ALT_CHARS {
            return Err(Error::InvalidOp("setAlt's text is longer than 2000 characters"));
        }
        // The file and page are judged before the name, so an untagged file says so rather than
        // that its object is gone.
        let (mcids, form) = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            let tagging = Tagging::read(doc, &self.page)?.ok_or(Error::InvalidOp("setAlt needs a tagged page"))?;
            let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(self.page.clone()))?;
            interpret_page(doc, &page, &mut ir.recorder(page.frame()?))?;
            let id = find(&ir, &self.page, self.object)?;
            let form = !structure::tagged_xobjects(doc, page.cos(), &ir.object_resource_names(id))?.is_empty();
            let figures = tagging.figures_of(doc, &ir.object_mcids(id))?;
            (figures, form)
        };
        let figure = match mcids.as_slice() {
            [one] => *one,
            [] if form => {
                return Err(Error::InvalidOp(
                    "setAlt can't reach the figures inside a form that is tagged itself",
                ));
            }
            [] => return Err(Error::InvalidOp("setAlt names an object that no figure owns")),
            _ => {
                return Err(Error::InvalidOp(
                    "setAlt names an object that belongs to more than one figure",
                ));
            }
        };
        let alt = Object::String(PdfString::literal(encode_text_string(&self.alt)));
        edit_dict(doc, figure, |d| {
            d.insert("Alt", alt);
        })?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            ..Effect::default()
        })
    }
}

/// A tagged page's structure: the tree (for the role map) and the page's slots in the parent tree.
struct Tagging {
    tree: laminapdf_model::StructTree,
    tags: PageTags,
}

impl Tagging {
    /// The page's tagging; `None` when the file isn't tagged or the page has no entry in the
    /// parent tree.
    fn read(doc: &laminapdf_model::Document, page: &PageId) -> Result<Option<Self>> {
        let Some(tree) = doc.structure()? else {
            return Ok(None);
        };
        Ok(PageTags::read(doc, page)?.map(|tags| Self { tree, tags }))
    }

    /// The `Figure`s that own `mcids` (see [`figure_of`]), each once, in the order found.
    fn figures_of(&self, doc: &laminapdf_model::Document, mcids: &BTreeSet<i64>) -> Result<Vec<ObjRef>> {
        let mut figures: Vec<ObjRef> = Vec::new();
        for owner in mcids.iter().filter_map(|&m| self.tags.owner(m)) {
            if let Some(figure) = figure_of(doc, &self.tree, owner)?
                && !figures.contains(&figure)
            {
                figures.push(figure);
            }
        }
        Ok(figures)
    }
}

/// The alternative text of the `Figure` each object of page `page` belongs to, for the objects
/// that belong to exactly one figure that has an `/Alt`: what [`SetAlt`] would replace. Empty
/// for an untagged file or page.
///
/// # Errors
/// Errors reading the structure tree.
///
/// Added in 0.7.0.
pub fn figure_alts(
    doc: &laminapdf_model::Document,
    page: &PageId,
    ir: &ContentOps,
) -> Result<Vec<(ContentObjectId, String)>> {
    let Some(tagging) = Tagging::read(doc, page)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for object in ir.objects() {
        let id = object.id();
        let figures = tagging.figures_of(doc, &ir.object_mcids(id))?;
        let [figure] = figures.as_slice() else {
            continue;
        };
        let alt = match object_dict(doc, *figure)?.and_then(|d| d.get("Alt").cloned()) {
            Some(value) => match &*doc.deref(&value)? {
                Object::String(s) => Some(decode_text_string(&s.bytes)),
                _ => None,
            },
            None => None,
        };
        if let Some(alt) = alt {
            out.push((id, alt));
        }
    }
    Ok(out)
}

/// The nearest `Figure` at or above `elem` (through the role map, as `tree` reads it).
fn figure_of(
    doc: &laminapdf_model::Document,
    tree: &laminapdf_model::StructTree,
    elem: ObjRef,
) -> Result<Option<ObjRef>> {
    let mut elem = elem;
    for _ in 0..=doc.limits().max_tree_depth() {
        let Some(dict) = object_dict(doc, elem)? else {
            return Ok(None);
        };
        if let Some(kind) = dict.get_name("S")
            && tree.standard_kind(kind) == b"Figure"
        {
            return Ok(Some(elem));
        }
        match dict.get("P").and_then(Object::as_ref) {
            Some(parent) => elem = parent,
            None => return Ok(None),
        }
    }
    Ok(None)
}
