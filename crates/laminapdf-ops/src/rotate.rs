//! `RotatePage`: set a page's `/Rotate` (ISO 32000-2 §7.7.3.3).

use std::sync::Arc;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{PageId, Rotation};

use crate::error::{Error, Result, tolerant};
use crate::wire;

/// Sets a page's rotation to `rotate` (absolute, not relative to the current value).
///
/// `/Rotate` is written on the page object itself, so a value inherited from a `/Pages`
/// node is materialised on this page and its siblings keep theirs; an existing own entry
/// keeps its position in the dictionary. The page object is changed, never replaced, so the
/// page keeps its [`PageId`] and object number ([ADR 0005] §5). Nothing is written when the
/// page already displays at `rotate` without an own entry.
///
/// On the wire: `{ "op": "rotatePage", "args": { "page": "p12.0", "rotate": 90 } }`.
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct RotatePage {
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The rotation in degrees clockwise: 0, 90, 180 or 270.
    #[serde(with = "wire::rotation")]
    #[cfg_attr(feature = "tsify", tsify(type = "0 | 90 | 180 | 270"))]
    pub rotate: Rotation,
}

impl RotatePage {
    /// The op that rotates `page` to `rotate`.
    #[must_use]
    pub const fn new(page: PageId, rotate: Rotation) -> Self {
        Self { page, rotate }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Vec<PageId>> {
        let (r, mut dict, inherited) = match self.find_by_parents(doc)? {
            Some(found) => found,
            None => self.find_by_walk(doc)?,
        };
        let degrees = Object::Integer(i64::from(self.rotate.degrees()));
        // An own value (even an invalid one, which this op repairs) is replaced in place;
        // without one, nothing is written when the inherited value already matches.
        let unchanged = match dict.get("Rotate") {
            Some(own) => *own == degrees,
            None => inherited == self.rotate,
        };
        if unchanged {
            return Ok(Vec::new());
        }
        dict.insert("Rotate", degrees);
        doc.cos_mut().edit().set(r, Object::Dict(dict))?;
        Ok(vec![self.page.clone()])
    }

    /// The page object, its dictionary and the rotation it inherits, found by following
    /// `/Parent` up to the root `/Pages` and checking each parent's `/Kids` lists the child:
    /// a few lookups instead of a page-tree walk. `None` when that path is broken (a wrong
    /// `/Parent`, a path ID, something unreadable): [`RotatePage::find_by_walk`] decides.
    fn find_by_parents(&self, doc: &laminapdf_model::Document) -> Result<Option<(ObjRef, Dict, Rotation)>> {
        let Some(r) = self.page.as_lamina_id().obj_ref() else {
            return Ok(None);
        };
        let Some(page) = tolerant(doc.resolve(r))? else {
            return Ok(None);
        };
        let Some(dict) = page.as_dict() else {
            return Ok(None);
        };
        let is_page = match dict.get_name("Type") {
            Some(t) => t == b"Page",
            None => !dict.contains_key("Kids"),
        };
        let root = tolerant(doc.catalog())?.and_then(|c| c.get("Pages").and_then(Object::as_ref));
        let (true, Some(root)) = (is_page, root) else {
            return Ok(None);
        };
        let mut inherited = None;
        let (mut child, mut node) = (r, Arc::clone(&page));
        for _ in 0..doc.limits().max_page_tree_depth() {
            let Some(parent_ref) = node.as_dict().and_then(|d| d.get("Parent")).and_then(Object::as_ref) else {
                return Ok(None);
            };
            let Some(parent) = tolerant(doc.resolve(parent_ref))? else {
                return Ok(None);
            };
            let Some(parent_dict) = parent.as_dict() else {
                return Ok(None);
            };
            let kids = match parent_dict.get("Kids") {
                Some(kids) => tolerant(doc.deref(kids).map(laminapdf_cos::Resolved::into_arc))?,
                None => None,
            };
            let listed = kids
                .as_deref()
                .and_then(Object::as_array)
                .is_some_and(|kids| kids.contains(&Object::Ref(child)));
            if !listed {
                return Ok(None);
            }
            if inherited.is_none()
                && let Some(value) = parent_dict.get("Rotate")
            {
                inherited = tolerant(doc.deref(value).map(laminapdf_cos::Resolved::into_owned))?
                    .filter(|v| !matches!(v, Object::Null));
            }
            if parent_ref == root {
                let rotation = inherited
                    .as_ref()
                    .and_then(Object::as_int)
                    .and_then(Rotation::from_degrees)
                    .unwrap_or(Rotation::None);
                return Ok(Some((r, dict.clone(), rotation)));
            }
            (child, node) = (parent_ref, parent);
        }
        Ok(None)
    }

    /// The page found by walking the page tree (the authority on what is a page); the
    /// displayed rotation is read without warnings, since this op replaces it.
    fn find_by_walk(&self, doc: &laminapdf_model::Document) -> Result<(ObjRef, Dict, Rotation)> {
        let pages = doc.pages()?;
        let page = pages
            .by_id(&self.page)
            .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
        let r = page.obj_ref().ok_or_else(|| Error::DirectPage(self.page.clone()))?;
        // The walk's own reading: own or inherited, a malformed value as 0, no warning.
        let displayed = Rotation::from_degrees(page.cos().rotate).unwrap_or(Rotation::None);
        Ok((r, page.dict().clone(), displayed))
    }
}
