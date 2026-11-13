//! `SetPageLabels`: replace the document's page labels, the catalog's `/PageLabels` number
//! tree (ISO 32000-2 §12.4.2).

use laminapdf_model::{LabelRange, LabelStyle, PageId, PageLabels};

use crate::error::{Error, Result};
use crate::wire;

/// Replaces the document's page labels with `ranges`; an empty list removes them, so viewers
/// show page numbers again.
///
/// Each range starts at a page, named by its ID, and labels that page and the ones after it up
/// to the next range. The ranges must be in page order, two never on one page, the first on
/// the document's first page (the number tree must have an entry for page 0, §12.4.2), and
/// each numbered from 1 or more; otherwise the op fails with `invalid-label-ranges`. Nothing
/// is written when the document already shows these labels.
///
/// Labels move with their pages afterwards: every page op rebuilds the tree from page IDs
/// ([`PageLabels::remap`]).
///
/// On the wire:
/// `{ "op": "setPageLabels", "args": { "ranges": [{ "page": "p3.0", "style": "lowerRoman" }] } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetPageLabels {
    /// The ranges, in page order.
    pub ranges: Vec<PageLabelRange>,
}

impl SetPageLabels {
    /// The op that gives the document these label `ranges`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(ranges: Vec<PageLabelRange>) -> Self {
        Self { ranges }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let pages = doc.pages()?;
        let mut ranges = Vec::with_capacity(self.ranges.len());
        let mut previous: Option<u64> = None;
        for range in &self.ranges {
            let index = pages
                .iter()
                .position(|p| *p.id() == range.page)
                .ok_or_else(|| Error::UnknownPage(range.page.clone()))?;
            let start = u64::try_from(index).unwrap_or(u64::MAX);
            match previous {
                None if start != 0 => return Err(invalid("first-page")),
                Some(before) if start <= before => return Err(invalid("order")),
                _ => {}
            }
            if range.first == 0 {
                return Err(invalid("first-number"));
            }
            previous = Some(start);
            ranges.push(LabelRange::new(start, range.style, range.prefix.clone(), range.first));
        }
        let labels = PageLabels::from_ranges(ranges);
        if doc.page_labels()? == labels {
            return Ok(());
        }
        labels.write(&mut doc.cos_mut().edit())?;
        Ok(())
    }
}

fn invalid(what: &'static str) -> Error {
    Error::InvalidLabelRanges { what }
}

/// One range of [`SetPageLabels`]: the labels from `page` up to the next range's page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PageLabelRange {
    /// The range's first page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The numbering style; without one, each label is the prefix alone. On the wire:
    /// `decimal`, `upperRoman`, `lowerRoman`, `upperLetters` or `lowerLetters`.
    #[serde(default, with = "wire::label_style", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"decimal\" | \"upperRoman\" | \"lowerRoman\" | \"upperLetters\" | \"lowerLetters\""
        )
    )]
    pub style: Option<LabelStyle>,
    /// Text before each number (`A-` in `A-1`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub prefix: String,
    /// The number of the range's first page, 1 or more.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub first: u64,
}

impl PageLabelRange {
    /// A range from `page` numbered in `style` from 1, without a prefix.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(page: PageId, style: Option<LabelStyle>) -> Self {
        Self {
            page,
            style,
            prefix: String::new(),
            first: 1,
        }
    }

    /// This range with `prefix` before each number.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// This range numbered from `first`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_first(mut self, first: u64) -> Self {
        self.first = first;
        self
    }
}

const fn one() -> u64 {
    1
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
const fn is_one(n: &u64) -> bool {
    *n == 1
}
