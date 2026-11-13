//! `InsertBlankPages`: new empty pages (ISO 32000-2 §7.7.3.3).

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{PageId, PagePosition, Rotation};

use super::{page_ids, pages_before, relabel};
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

/// A page size: the width and height of the `/MediaBox`, in points (1/72 inch).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PageSize {
    /// The width in points.
    pub width: f64,
    /// The height in points.
    pub height: f64,
}

impl PageSize {
    /// A page `width` by `height` points.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// US Letter, 612 by 792 points.
    ///
    /// Added in 0.2.0.
    pub const LETTER: Self = Self::new(612.0, 792.0);

    /// A4, 595.28 by 841.89 points.
    ///
    /// Added in 0.2.0.
    pub const A4: Self = Self::new(595.28, 841.89);
}

impl Eq for PageSize {}

/// Inserts `count` blank pages of `size` at `at`, each showing at `rotate`. Each new page
/// gets a new object and [`PageId`] (ADR 0005 §5), an empty `/Resources` and no content; in
/// a labelled document they continue the label range they land in.
///
/// On the wire: `{ "op": "insertBlankPages", "args": { "at": { "after": "p3.0" }, "count": 2,
/// "size": { "width": 612, "height": 792 }, "rotate": 0 } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct InsertBlankPages {
    /// Where the pages go.
    #[serde(with = "wire::position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "\"start\" | \"end\" | { before: string } | { after: string }")
    )]
    pub at: PagePosition,
    /// How many pages, at least 1.
    pub count: u32,
    /// Their `/MediaBox` size.
    pub size: PageSize,
    /// Their rotation in degrees clockwise: 0, 90, 180 or 270.
    #[serde(with = "wire::rotation")]
    #[cfg_attr(feature = "tsify", tsify(type = "0 | 90 | 180 | 270"))]
    pub rotate: Rotation,
}

impl InsertBlankPages {
    /// The op that inserts `count` pages of `size` at `at`, upright.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(at: PagePosition, count: u32, size: PageSize) -> Self {
        Self {
            at,
            count,
            size,
            rotate: Rotation::None,
        }
    }

    /// With the pages shown at `rotate`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_rotate(mut self, rotate: Rotation) -> Self {
        self.rotate = rotate;
        self
    }

    /// The page dictionary each new page starts from.
    fn page(&self) -> Dict {
        let mut dict = Dict::new();
        let size = |v: f64| Object::Real(if v.is_finite() && v > 0.0 { v } else { 1.0 });
        dict.insert(
            "MediaBox",
            Object::Array(vec![0.into(), 0.into(), size(self.size.width), size(self.size.height)]),
        );
        dict.insert("Resources", Object::Dict(Dict::new()));
        if self.rotate != Rotation::None {
            dict.insert("Rotate", Object::Integer(i64::from(self.rotate.degrees())));
        }
        dict
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        if self.count == 0 {
            return Ok(Effect::default());
        }
        let before = pages_before(doc)?;
        let mut added: Vec<PageId> = Vec::new();
        {
            let mut editor = doc.edit_page_tree()?;
            let mut at = self.at.clone();
            for _ in 0..self.count {
                let id = editor.insert_new(self.page(), &at)?;
                at = PagePosition::After(id.clone());
                added.push(id);
            }
            editor.finish()?;
        }
        let after = page_ids(doc)?;
        relabel(doc, &before, &after)?;
        Ok(Effect {
            pages: added,
            page_order: Some(after),
            ..Effect::default()
        })
    }
}
