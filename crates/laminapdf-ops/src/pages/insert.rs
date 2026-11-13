//! `InsertBlankPages`: new empty pages (ISO 32000-2 §7.7.3.3).

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{PageId, PagePosition, PageSize, Rotation};

use super::{page_ids, pages_before, relabel};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// Inserts `count` blank pages of `size` at `at`, each showing at `rotate`. Each new page
/// gets a new object and [`PageId`] (ADR 0005 §5), an empty `/Resources` and no content; in
/// a labelled document they continue the label range they land in. A count outside 1 to
/// 10,000, or a side that isn't above 0 and at most 14,400 points, is
/// [`Error::InvalidOp`](crate::Error::InvalidOp).
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
// to itself, NaN, never reaches an op from JSON, and is refused as `invalid-op`.
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
        let size = |v: laminapdf_model::Pt| Object::Real(v.get());
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

    /// The most pages one op inserts.
    const MAX_COUNT: u32 = 10_000;

    /// The largest page side, in points: 14,400 units at a `/UserUnit` of 1 (ISO 32000-2
    /// Annex C).
    const MAX_SIDE: f64 = 14_400.0;

    fn check(&self) -> Result<()> {
        if self.count == 0 || self.count > Self::MAX_COUNT {
            return Err(Error::InvalidOp("insert between 1 and 10,000 pages"));
        }
        let side = |v: laminapdf_model::Pt| v.get().is_finite() && v.get() > 0.0 && v.get() <= Self::MAX_SIDE;
        if !side(self.size.width) || !side(self.size.height) {
            return Err(Error::InvalidOp(
                "a page's width and height are above 0 and at most 14,400 points",
            ));
        }
        Ok(())
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        self.check()?;
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
