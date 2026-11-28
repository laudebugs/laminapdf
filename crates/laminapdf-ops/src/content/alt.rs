//! `SetAlt`: the alternative text of the figure a page's image belongs to, so a reader who
//! selected the image can describe it for a screen reader (ISO 32000-2 §14.9.3).

use laminapdf_content::{ContentOps, ContentOwner, interpret_page};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_syntax::PdfString;
use laminapdf_syntax::text::encode_text_string;
use serde::{Deserialize, Serialize};

use super::structure::PageTags;
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
/// The file must be tagged, and `object` must be drawn inside marked content that a `Figure`
/// owns: an untagged file, an object outside every marked-content sequence and an object no
/// `Figure` owns are refused with `invalid-op`. The page's content isn't changed. A name whose
/// hash no longer matches fails with `content-object-stale`.
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
        let mcids = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(self.page.clone()))?;
            interpret_page(doc, &page, &mut ir.recorder(page.frame()?))?;
            let id = find(&ir, &self.page, self.object)?;
            ir.object_mcids(id)
        };
        let tree = doc.structure()?.ok_or(Error::InvalidOp("setAlt needs a tagged file"))?;
        let tags = PageTags::read(doc, &self.page)?.ok_or(Error::InvalidOp("setAlt needs a tagged page"))?;
        if mcids.is_empty() {
            return Err(Error::InvalidOp("setAlt names an object that isn't tagged"));
        }
        let mut figures: Vec<ObjRef> = Vec::new();
        for owner in mcids.iter().filter_map(|&m| tags.owner(m)) {
            if let Some(figure) = figure_of(doc, &tree, owner)?
                && !figures.contains(&figure)
            {
                figures.push(figure);
            }
        }
        if figures.is_empty() {
            return Err(Error::InvalidOp("setAlt names an object that no figure owns"));
        }
        let alt = Object::String(PdfString::literal(encode_text_string(&self.alt)));
        for figure in figures {
            edit_dict(doc, figure, |d| {
                d.insert("Alt", alt.clone());
            })?;
        }
        Ok(Effect {
            pages: vec![self.page.clone()],
            ..Effect::default()
        })
    }
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
