//! Page labels (§12.4.2): the catalog's `/PageLabels` number tree.

use lamina_cos::{Dict, Name, Object, PdfString};
use lamina_model::LabelStyle;
use lamina_syntax::text::encode_text_string;

use crate::error::{Error, Result};

/// One page label range: pages from `start` up to the next range's start are labelled
/// `prefix` followed by their number in `style`, counting from `first`.
///
/// ```
/// use lamina_build::{LabelStyle, PageLabel};
///
/// // i, ii, iii, … then 1, 2, … from the fourth page, then A-1, A-2, … from the tenth.
/// let labels = [
///     PageLabel::new(0, Some(LabelStyle::LowerRoman)),
///     PageLabel::new(3, Some(LabelStyle::Decimal)),
///     PageLabel::new(9, Some(LabelStyle::Decimal)).with_prefix("A-"),
/// ];
/// assert_eq!(labels[2].prefix(), "A-");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageLabel {
    start: u32,
    style: Option<LabelStyle>,
    prefix: String,
    first: u32,
}

impl PageLabel {
    /// A range from page `start` (0-based) numbered in `style` from 1; `None`: labels are the
    /// prefix alone.
    #[must_use]
    pub const fn new(start: u32, style: Option<LabelStyle>) -> Self {
        Self {
            start,
            style,
            prefix: String::new(),
            first: 1,
        }
    }

    /// With a prefix (`/P`).
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Numbered from `first` (`/St`, at least 1).
    #[must_use]
    pub const fn with_first(mut self, first: u32) -> Self {
        self.first = first;
        self
    }

    /// The range's first page index.
    #[must_use]
    pub const fn start(&self) -> u32 {
        self.start
    }

    /// The numbering style.
    #[must_use]
    pub const fn style(&self) -> Option<LabelStyle> {
        self.style
    }

    /// The prefix.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// The first number.
    #[must_use]
    pub const fn first(&self) -> u32 {
        self.first
    }
}

const fn style_name(style: LabelStyle) -> &'static str {
    match style {
        LabelStyle::Decimal => "D",
        LabelStyle::UpperRoman => "R",
        LabelStyle::LowerRoman => "r",
        LabelStyle::UpperLetters => "A",
        LabelStyle::LowerLetters => "a",
    }
}

/// The `/PageLabels` number tree (one leaf) for a document of `pages` pages.
pub(crate) fn labels_tree(ranges: &[PageLabel], pages: usize) -> Result<Dict> {
    let invalid = |what| Error::InvalidPageLabels { what };
    if ranges.first().is_none_or(|r| r.start != 0) {
        return Err(invalid("the first range must start at page index 0"));
    }
    if ranges.windows(2).any(|w| matches!(w, [a, b] if a.start >= b.start)) {
        return Err(invalid("ranges must start at increasing page indexes"));
    }
    if ranges
        .iter()
        .any(|r| usize::try_from(r.start).is_ok_and(|start| start >= pages.max(1)))
    {
        return Err(invalid("a range starts past the last page"));
    }
    if ranges.iter().any(|r| r.first == 0) {
        return Err(invalid("label numbers start at 1 or more"));
    }
    let mut nums = Vec::with_capacity(ranges.len() * 2);
    for r in ranges {
        let mut d = Dict::new();
        if let Some(style) = r.style {
            d.insert("S", Name::from(style_name(style)));
        }
        if !r.prefix.is_empty() {
            d.insert("P", PdfString::literal(encode_text_string(&r.prefix)));
        }
        if r.first != 1 {
            d.insert("St", r.first);
        }
        nums.push(Object::from(r.start));
        nums.push(Object::Dict(d));
    }
    let mut tree = Dict::new();
    tree.insert("Nums", Object::Array(nums));
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_must_start_at_zero() {
        let e = labels_tree(&[PageLabel::new(1, None)], 3).unwrap_err();
        assert_eq!(e.code(), "invalid-page-labels");
    }

    #[test]
    fn ranges_must_increase() {
        let ranges = [
            PageLabel::new(0, None),
            PageLabel::new(2, None),
            PageLabel::new(2, None),
        ];
        assert!(labels_tree(&ranges, 3).is_err());
    }

    #[test]
    fn default_entries_are_left_out() {
        let tree = labels_tree(&[PageLabel::new(0, Some(LabelStyle::Decimal))], 1).unwrap();
        let mut d = Dict::new();
        d.insert("S", Name::from("D"));
        assert_eq!(tree.get("Nums"), Some(&Object::Array(vec![0.into(), d.into()])));
    }
}
