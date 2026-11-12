//! Page labels (ISO 32000-2 §12.4.2): the catalog's `/PageLabels` number tree, from page
//! index to label ranges.

use std::collections::BTreeMap;

use laminapdf_cos::{CosMut, Dict, Object, PdfString, Result, Warning};
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use crate::document::Document;
use crate::edit::set_catalog_entry;
use crate::id::PageId;
use crate::tree::NumberTree;
use crate::tree_writer::TreeWriter;

/// The numbering style of a label range (`/S`, Table 161).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelStyle {
    /// `D`: decimal arabic numerals.
    Decimal,
    /// `R`: uppercase roman numerals.
    UpperRoman,
    /// `r`: lowercase roman numerals.
    LowerRoman,
    /// `A`: uppercase letters (A to Z, then AA to ZZ, …).
    UpperLetters,
    /// `a`: lowercase letters.
    LowerLetters,
}

impl LabelStyle {
    const fn name(self) -> &'static str {
        match self {
            Self::Decimal => "D",
            Self::UpperRoman => "R",
            Self::LowerRoman => "r",
            Self::UpperLetters => "A",
            Self::LowerLetters => "a",
        }
    }

    fn from_name(name: &[u8]) -> Option<Self> {
        Some(match name {
            b"D" => Self::Decimal,
            b"R" => Self::UpperRoman,
            b"r" => Self::LowerRoman,
            b"A" => Self::UpperLetters,
            b"a" => Self::LowerLetters,
            _ => return None,
        })
    }

    /// The numeric part of a label for `value` (at least 1).
    ///
    /// Roman numerals and letters grow linearly with the value, so values above
    /// [`MAX_SPELLED`] are written in decimal instead: that keeps a hostile `/St` from
    /// producing a huge string.
    fn format(self, value: u64) -> String {
        if value > MAX_SPELLED && self != Self::Decimal {
            return value.to_string();
        }
        match self {
            Self::Decimal => value.to_string(),
            Self::UpperRoman => roman(value),
            Self::LowerRoman => roman(value).to_ascii_lowercase(),
            Self::UpperLetters => letters(value, b'A'),
            Self::LowerLetters => letters(value, b'a'),
        }
    }
}

/// Roman numerals and letters are spelled out up to this value (4,000 roman numerals need
/// at most 4 `M`s plus 12 more characters; letters at most 385 characters).
const MAX_SPELLED: u64 = 10_000;

fn roman(mut value: u64) -> String {
    const TABLE: [(u64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (n, s) in TABLE {
        while value >= n {
            out.push_str(s);
            value -= n;
        }
    }
    out
}

/// §12.4.2: A to Z for 1–26, AA to ZZ for 27–52, AAA to ZZZ for 53–78, and so on.
fn letters(value: u64, base: u8) -> String {
    // Values start at 1; 0 (never produced: `/St` is at least 1) reads as 1.
    let zero_based = value.saturating_sub(1);
    let index = zero_based % 26;
    let repeat = zero_based / 26 + 1;
    let letter = char::from(base + u8::try_from(index).unwrap_or(0));
    std::iter::repeat_n(letter, usize::try_from(repeat).unwrap_or(0)).collect()
}

/// One label range: the pages from `start` up to the next range's start.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LabelRange {
    /// The index of the range's first page.
    pub start: u64,
    /// The numbering style; `None` means labels are the prefix only.
    pub style: Option<LabelStyle>,
    /// The label prefix (`/P`), decoded.
    pub prefix: String,
    /// The number of the range's first page (`/St`, default 1).
    pub first: u64,
}

impl LabelRange {
    /// The label dictionary (§12.4.2, Table 161), leaving out `/St 1` and an empty prefix.
    fn to_dict(&self) -> Dict {
        let mut dict = Dict::new();
        if let Some(style) = self.style {
            dict.insert("S", Object::name(style.name()));
        }
        if !self.prefix.is_empty() {
            dict.insert(
                "P",
                Object::String(PdfString::literal(encode_text_string(&self.prefix))),
            );
        }
        if self.first != 1 {
            dict.insert("St", Object::Integer(i64::try_from(self.first).unwrap_or(i64::MAX)));
        }
        dict
    }

    fn label(&self, index: u64) -> String {
        let mut out = self.prefix.clone();
        if let Some(style) = self.style {
            let value = self.first.saturating_add(index.saturating_sub(self.start));
            out.push_str(&style.format(value));
        }
        out
    }
}

/// A document's page labels: the ranges of `/PageLabels`, sorted by first page.
///
/// ```
/// use laminapdf_model::{LabelRange, PageLabels};
///
/// let labels = PageLabels::default();
/// assert_eq!(labels.label(0), None); // no /PageLabels: no labels
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageLabels {
    ranges: Vec<LabelRange>,
}

impl PageLabels {
    pub(crate) fn new(doc: &Document) -> Result<Self> {
        let catalog = doc.catalog()?;
        let Some(root) = catalog.get("PageLabels") else {
            return Ok(Self::default());
        };
        let mut ranges: Vec<LabelRange> = Vec::new();
        for (start, value) in NumberTree::new(doc, root.clone()).entries()? {
            let Ok(index) = u64::try_from(start) else {
                doc.warn(Warning::InvalidPageLabel { start })?;
                continue;
            };
            ranges.push(range(doc, start, index, &value)?);
        }
        // Tree order should already be sorted; keep the first range for a repeated start.
        ranges.sort_by_key(|r| r.start);
        ranges.dedup_by_key(|r| r.start);
        Ok(Self { ranges })
    }

    /// The ranges, sorted by first page.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn ranges(&self) -> &[LabelRange] {
        &self.ranges
    }

    /// True when the document has no page labels.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// The label of the page at `index`; `None` without page labels or before the first
    /// range.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn label(&self, index: usize) -> Option<String> {
        let index = u64::try_from(index).ok()?;
        let at = self.ranges.partition_point(|r| r.start <= index);
        let range = self.ranges.get(at.checked_sub(1)?)?;
        Some(range.label(index))
    }

    /// The labels after a change to the page list: `before` is the list these labels
    /// describe, `after` the new one (each a list of page IDs, in order). Every page in both
    /// keeps its label; a page only in `after` continues the range it lands in (the label
    /// after its previous page's; new pages at the start lead into the first page's range,
    /// numbered down to 1). Ranges are then written again from the labels, so a range
    /// starts wherever the style, the prefix or the run of numbers breaks.
    ///
    /// A page that had no label (before the first range) keeps none while it stays ahead of
    /// every labelled page; one that lands after a labelled page gets the decimal number it
    /// was shown before, its old index plus one.
    ///
    /// ```
    /// use laminapdf_model::{PageId, PageLabels};
    /// # use laminapdf_cos::ObjRef;
    /// let ids: Vec<PageId> = (1..=4).map(|n| PageId::object(ObjRef::new(n, 0))).collect();
    /// let labels = PageLabels::default(); // no /PageLabels
    /// let moved = vec![ids[3].clone(), ids[0].clone(), ids[1].clone(), ids[2].clone()];
    /// assert!(labels.remap(&ids, &moved).is_empty());
    /// ```
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn remap(&self, before: &[PageId], after: &[PageId]) -> Self {
        if self.ranges.is_empty() {
            return Self::default();
        }
        let old: BTreeMap<&PageId, usize> = before.iter().enumerate().map(|(i, id)| (id, i)).collect();
        let was: Vec<Was> = after
            .iter()
            .map(|id| match old.get(id) {
                Some(&i) => self.spec(i).map_or_else(|| Was::Unlabelled(i), Was::Label),
                None => Was::New,
            })
            .collect();
        let specs = fill(&was);
        Self::from_specs(&specs)
    }

    /// Writes these labels as the catalog's `/PageLabels` number tree, or removes the entry
    /// when there are none.
    ///
    /// # Errors
    /// Errors of the overlay; [`laminapdf_cos::Error::MissingRoot`] without a catalog.
    ///
    /// Added in 0.2.0.
    pub fn write(&self, cos: &mut CosMut<'_>) -> Result<()> {
        if self.ranges.is_empty() {
            return set_catalog_entry(cos, "PageLabels", None);
        }
        let entries: BTreeMap<i64, Object> = self
            .ranges
            .iter()
            .map(|r| (i64::try_from(r.start).unwrap_or(i64::MAX), Object::Dict(r.to_dict())))
            .collect();
        let root = TreeWriter::new().write_numbers(cos, &entries)?;
        set_catalog_entry(cos, "PageLabels", Some(Object::Ref(root)))
    }

    /// What page `index` shows: its label's parts, `None` before the first range.
    fn spec(&self, index: usize) -> Option<Spec> {
        let index = u64::try_from(index).ok()?;
        let at = self.ranges.partition_point(|r| r.start <= index);
        let Some(range) = at.checked_sub(1).and_then(|i| self.ranges.get(i)) else {
            // Unlabelled: a viewer shows the page number.
            return None;
        };
        Some(Spec {
            style: range.style,
            prefix: range.prefix.clone(),
            value: range.first.saturating_add(index - range.start),
        })
    }

    /// The fewest ranges that give each page its label; unlabelled pages lead the list.
    fn from_specs(specs: &[Option<Spec>]) -> Self {
        let mut ranges: Vec<LabelRange> = Vec::new();
        let mut previous: Option<Spec> = None;
        for (index, spec) in specs.iter().enumerate() {
            let start = u64::try_from(index).unwrap_or(u64::MAX);
            let Some(spec) = spec.clone() else {
                previous = None;
                continue;
            };
            if previous.as_ref().is_none_or(|p| !p.continued_by(&spec)) {
                ranges.push(LabelRange {
                    start,
                    style: spec.style,
                    prefix: spec.prefix.clone(),
                    first: spec.value,
                });
            }
            previous = Some(spec);
        }
        Self { ranges }
    }
}

/// The parts of one page's label.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Spec {
    style: Option<LabelStyle>,
    prefix: String,
    /// The number (unused without a style).
    value: u64,
}

impl Spec {
    /// Whether `next`, on the following page, can stay in this page's range.
    fn continued_by(&self, next: &Self) -> bool {
        self.style == next.style
            && self.prefix == next.prefix
            && (self.style.is_none() || self.value.checked_add(1) == Some(next.value))
    }

    fn next(&self) -> Self {
        Self {
            value: self.value.saturating_add(1),
            ..self.clone()
        }
    }
}

/// What a page of the new list was in the old one.
enum Was {
    /// A labelled page.
    Label(Spec),
    /// An unlabelled page (before the first range), at this old index.
    Unlabelled(usize),
    /// A page the old list didn't have.
    New,
}

/// Each page's label in the new list. A new page continues the labelled page before it; an
/// old unlabelled page after a labelled one gets the decimal number a viewer showed for it.
/// Pages ahead of every labelled page stay unlabelled, except new pages right before the
/// first labelled one, which lead into its range, numbered down to 1.
fn fill(was: &[Was]) -> Vec<Option<Spec>> {
    let mut out: Vec<Option<Spec>> = Vec::with_capacity(was.len());
    let mut previous: Option<Spec> = None;
    for page in was {
        let spec = match (page, &previous) {
            (Was::Label(spec), _) => Some(spec.clone()),
            (Was::Unlabelled(index), Some(_)) => Some(Spec {
                style: Some(LabelStyle::Decimal),
                prefix: String::new(),
                value: u64::try_from(*index).unwrap_or(u64::MAX).saturating_add(1),
            }),
            (Was::New, Some(p)) => Some(p.next()),
            (Was::Unlabelled(_) | Was::New, None) => None,
        };
        if spec.is_some() {
            previous.clone_from(&spec);
        }
        out.push(spec);
    }
    let Some(first) = out.iter().position(Option::is_some) else {
        return out;
    };
    let Some(Some(lead)) = out.get(first).cloned() else {
        return out;
    };
    let mut value = lead.value;
    for (slot, page) in out.iter_mut().zip(was).take(first).rev() {
        if !matches!(page, Was::New) {
            break;
        }
        // Numbers start at 1 (§12.4.2, `/St`): below that the pages repeat it.
        value = value.saturating_sub(1).max(1);
        *slot = Some(Spec { value, ..lead.clone() });
    }
    out
}

/// The range starting at page `index` (stored as `start`) from its label dictionary. A value
/// that is not a dictionary falls back to decimal numbering from 1; a malformed entry is
/// dropped on its own (`/S`: no numeric part, `/P`: no prefix, `/St`: from 1), keeping the
/// others. Each raises [`Warning::InvalidPageLabel`].
fn range(doc: &Document, start: i64, index: u64, value: &Object) -> Result<LabelRange> {
    let fallback = LabelRange {
        start: index,
        style: Some(LabelStyle::Decimal),
        prefix: String::new(),
        first: 1,
    };
    let invalid = || doc.warn(Warning::InvalidPageLabel { start });
    let Some(dict) = doc.read(value)? else {
        invalid()?;
        return Ok(fallback);
    };
    let Some(dict) = dict.as_dict() else {
        invalid()?;
        return Ok(fallback);
    };
    let style = match dict.get("S") {
        None => None,
        Some(s) => {
            let style = match doc.read(s)?.as_deref() {
                Some(Object::Name(n)) => LabelStyle::from_name(n.as_bytes()),
                _ => None,
            };
            if style.is_none() {
                invalid()?;
            }
            style
        }
    };
    let prefix = match dict.get("P") {
        Some(p) => {
            if let Some(Object::String(s)) = doc.read(p)?.as_deref() {
                decode_text_string(&s.bytes)
            } else {
                invalid()?;
                String::new()
            }
        }
        None => String::new(),
    };
    let first = match dict.get("St") {
        Some(st) => match doc
            .read(st)?
            .and_then(|n| n.as_int())
            .and_then(|n| u64::try_from(n).ok())
        {
            Some(n) if n >= 1 => n,
            _ => {
                invalid()?;
                1
            }
        },
        None => 1,
    };
    Ok(LabelRange {
        start: index,
        style,
        prefix,
        first,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roman_numerals_follow_the_subtractive_rule() {
        let got: Vec<String> = [1, 4, 9, 14, 40, 90, 400, 1994, 3999].map(roman).to_vec();
        assert_eq!(got, ["I", "IV", "IX", "XIV", "XL", "XC", "CD", "MCMXCIV", "MMMCMXCIX"]);
    }

    #[test]
    fn letters_repeat_after_z() {
        let got: Vec<String> = [1, 26, 27, 52, 53].map(|v| letters(v, b'A')).to_vec();
        assert_eq!(got, ["A", "Z", "AA", "ZZ", "AAA"]);
    }

    #[test]
    fn letters_of_zero_do_not_underflow() {
        assert_eq!(letters(0, b'a'), "a");
    }

    #[test]
    fn a_page_before_its_range_start_does_not_underflow() {
        let range = LabelRange {
            start: 5,
            style: Some(LabelStyle::Decimal),
            prefix: String::new(),
            first: 1,
        };
        assert_eq!(range.label(2), "1");
    }

    #[test]
    fn huge_values_are_written_in_decimal() {
        assert_eq!(LabelStyle::UpperLetters.format(u64::MAX), u64::MAX.to_string());
    }

    #[test]
    fn label_uses_the_last_range_starting_at_or_before_the_page() {
        let labels = PageLabels {
            ranges: vec![
                LabelRange {
                    start: 0,
                    style: Some(LabelStyle::LowerRoman),
                    prefix: String::new(),
                    first: 1,
                },
                LabelRange {
                    start: 3,
                    style: Some(LabelStyle::Decimal),
                    prefix: "A-".into(),
                    first: 8,
                },
            ],
        };
        let got: Vec<_> = (0..5).map(|i| labels.label(i).unwrap()).collect();
        assert_eq!(got, ["i", "ii", "iii", "A-8", "A-9"]);
    }

    #[test]
    fn label_before_the_first_range_is_none() {
        let labels = PageLabels {
            ranges: vec![LabelRange {
                start: 2,
                style: None,
                prefix: "cover".into(),
                first: 1,
            }],
        };
        assert_eq!((labels.label(1), labels.label(2)), (None, Some("cover".into())));
    }
}
