//! `RotatePage`: set a page's `/Rotate` (ISO 32000-2 §7.7.3.3).

use lamina_cos::Object;
use lamina_model::{PageId, Rotation};

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
/// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
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

    pub(crate) fn apply_to(&self, doc: &mut lamina_model::Document) -> Result<Vec<PageId>> {
        let (r, displayed) = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            let r = page.obj_ref().ok_or_else(|| Error::DirectPage(self.page.clone()))?;
            (r, page.rotate()?)
        };
        let mut cos = doc.cos_mut().edit();
        let Some(Object::Dict(mut dict)) = cos.get(r)?.as_deref().cloned() else {
            // The walk found a page dictionary at `r`, so this is unreachable from input;
            // report it like any other lookup of a missing page.
            return Err(Error::UnknownPage(self.page.clone()));
        };
        if !dict.contains_key("Rotate") && displayed == self.rotate {
            return Ok(Vec::new());
        }
        let degrees = Object::Integer(i64::from(self.rotate.degrees()));
        if dict.get("Rotate") == Some(&degrees) {
            return Ok(Vec::new());
        }
        dict.insert("Rotate", degrees);
        cos.set(r, Object::Dict(dict))?;
        Ok(vec![self.page.clone()])
    }
}
