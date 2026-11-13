//! `InsertBlankPages`: new empty pages (ISO 32000-2 §7.7.3.3).

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{PageId, PagePosition, PageSize, Rotation};

use super::{page_ids, pages_before, relabel};
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

/// Inserts `count` blank pages of `size` at `at`, each showing at `rotate`. Each new page
/// gets a new object and [`PageId`] (ADR 0005 §5), an empty `/Resources` and no content; in
/// a labelled document they continue the label range they land in.
///
/// On the wire: `{ "op": "insertBlankPages", "args": { "at": { "after": "p3.0" }, "count": 2,
/// "size": { "width": 612, "height": 792 }, "rotate": 0 } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Their `/MediaBox` size, in points: `{ "width": 612, "height": 792 }`.
    #[serde(with = "wire::page_size")]
    #[cfg_attr(feature = "tsify", tsify(type = "{ width: number; height: number }"))]
    pub size: PageSize,
    /// Their rotation in degrees clockwise: 0, 90, 180 or 270.
    #[serde(with = "wire::rotation")]
    #[cfg_attr(feature = "tsify", tsify(type = "0 | 90 | 180 | 270"))]
    pub rotate: Rotation,
}

// Ops compare by value (`Op: Eq`); a size is two numbers, and the one value that isn't equal
// to itself, NaN, never reaches an op from JSON, and would be written as a 1-point page.
impl Eq for InsertBlankPages {}

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
        let size = |v: laminapdf_model::Pt| {
            let v = v.get();
            Object::Real(if v.is_finite() && v > 0.0 { v } else { 1.0 })
        };
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
