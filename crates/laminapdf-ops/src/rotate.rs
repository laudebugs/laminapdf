//! `RotatePage`: set a page's `/Rotate` (ISO 32000-2 §7.7.3.3).

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{PageId, Rotation};

use crate::error::{Error, Result};
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
        // An own value is replaced in place unless it already states this rotation (`90`,
        // `90.0` or a reference to either); an invalid one is repaired. Without one (or with
        // `null`, which inherits), nothing is written when the inherited value already matches.
        let unchanged = match dict.get("Rotate").filter(|own| !matches!(own, Object::Null)) {
            Some(own) => doc.read_rotate(own)? == Some(self.rotate),
            None => inherited == self.rotate,
        };
        if unchanged {
            return Ok(Vec::new());
        }
        dict.insert("Rotate", degrees);
        doc.cos_mut().edit().set(r, Object::Dict(dict))?;
        Ok(vec![self.page.clone()])
    }

    /// The page object, its dictionary and the rotation it inherits, from
    /// [`laminapdf_model::Document::page_by_parents`]: a few lookups instead of a page-tree
    /// walk. `None` when that path is broken: [`RotatePage::find_by_walk`] decides.
    fn find_by_parents(&self, doc: &laminapdf_model::Document) -> Result<Option<(ObjRef, Dict, Rotation)>> {
        let Some(r) = self.page.as_lamina_id().obj_ref() else {
            return Ok(None);
        };
        Ok(doc
            .page_by_parents(r)?
            .map(|found| (r, found.dict, found.inherited_rotate)))
    }

    /// The page found by walking the page tree (the authority on what is a page); the
    /// displayed rotation is read without warnings, since this op replaces it.
    fn find_by_walk(&self, doc: &laminapdf_model::Document) -> Result<(ObjRef, Dict, Rotation)> {
        let pages = doc.pages()?;
        let page = pages
            .by_id(&self.page)
            .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
        let r = page.obj_ref().ok_or_else(|| Error::DirectPage(self.page.clone()))?;
        Ok((r, page.dict().clone(), page.rotate_quietly()?))
    }
}
