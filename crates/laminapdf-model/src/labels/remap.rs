//! Rewriting the page labels after a change to the page list ([`PageLabels::remap`]) and
//! writing them back as the catalog's `/PageLabels` number tree ([`PageLabels::write`]).

use std::collections::BTreeMap;

use laminapdf_cos::{CosMut, Dict, Object, PdfString, Result};
use laminapdf_syntax::text::encode_text_string;

use super::{LabelRange, LabelStyle, PageLabels};
use crate::edit::set_catalog_entry;
use crate::id::PageId;
use crate::tree_writer::TreeWriter;

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
}

impl PageLabels {
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
