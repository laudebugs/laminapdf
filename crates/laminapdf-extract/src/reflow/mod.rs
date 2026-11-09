//! Reader mode: a tagged document as reflowable blocks in the structure tree's order,
//! or a refusal when its tree can't be trusted.
//!
//! The text comes from extraction with [`ReadingOrder::Auto`], whose blocks already follow the
//! tree on each page where it passes ADR 0030 §2's checks, with headings, list items, captions,
//! notes and tables typed by their elements (ISO 32000-2 §14.8.4, through the `RoleMap`) and
//! running heads and feet dropped. Reader mode then:
//!
//! - shows each `Figure`, `Formula` and `Table` (§14.8.4.7–8) as a crop of its page with its
//!   `/Alt` (§14.9.3), never as reflowed text: its box is what its marked content paints, text and
//!   images and paths, since the builders that write a `BBox` attribute (§14.8.5.4.3) are few;
//!   a caption stays text even inside one;
//! - marks the paragraphs inside a `BlockQuote` as quotes;
//! - refuses a document whose tree is missing or empty, whose content sits mostly in no
//!   block-level element (bare `Span`s and `NonStruct`s), which the per-page checks ignored on
//!   too many pages, or whose order disagrees with the layout's (a rank correlation between the
//!   tree's order and XY-Cut++'s, which a reversed single column doesn't escape the per-page
//!   check's column jumps). The thresholds are calibrated on `fixtures/reflow/` (the module's
//!   tests and `tests/reflow.rs` hold them to it).
//!
//! Role mapping: every element's type is followed through the `RoleMap` to its standard type
//! before any rule reads it. `H1`–`H6`, `H` and `Title` are headings; `P`, `TOCI` and `Lbl`
//! paragraphs; `LI` a list item at its `L` depth; `Caption`; `Note` and `FENote` notes;
//! `BlockQuote` a quote around its paragraphs; `Figure`, `Formula` and `Table` crops; the
//! grouping types (`Document`, `Part`, `Sect`, `Div`, `Art`, `NonStruct`, …) say nothing.

mod order;
mod tree;

use std::collections::BTreeMap;

use laminapdf_cos::ObjRef;
use laminapdf_model::{Document, PageId, RectPt};
use laminapdf_syntax::{Budget, CancelToken, Step};

use crate::error::ExtractError;
use crate::job::{PageFeed, extract_job};
use crate::model::{Block, BlockKind};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::page::TreeUse;
use crate::tagged::Implausible;
use tree::{CropKind, TreeFacts};

/// The most content a tree may hold outside any block-level element.
const MAX_GENERIC: f64 = 0.5;

/// The most pages with tagged content whose tree the per-page checks may ignore.
const MAX_IGNORED: f64 = 0.25;

/// The least agreement between the tree's order and the layout's (Kendall's τ, −1 to 1).
/// Calibrated on `fixtures/reflow/`: every correct tree scores 1 but `pull-quote`'s, 0.52 (a
/// layout XY-Cut++ itself misreads); the shuffled trees score 0.43, 0.2 and 0.2, the reversed
/// ones −1. The margin is thin, so a correct tree on a page XY-Cut++ reads badly can be refused.
pub(crate) const MIN_AGREEMENT: f64 = 0.5;

/// What reader mode makes of a document.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Reflow {
    /// Whether reader mode can show the document; when it can't, `reason` says why and
    /// `blocks` is empty.
    pub available: bool,
    /// Why it can't; `None` when it can.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub reason: Option<ReflowRefusal>,
    /// The document's language (the catalog's `/Lang`), for the reading view's `lang`.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub lang: Option<String>,
    /// The blocks, in reading order.
    pub blocks: Vec<ReflowBlock>,
    /// The measures the checks read.
    pub checks: ReflowChecks,
}

/// Why reader mode refuses a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum ReflowRefusal {
    /// It isn't tagged (no `/MarkInfo /Marked true`, or no structure tree).
    Untagged,
    /// Its tree holds no marked content on its pages.
    Empty,
    /// Most of its content sits in no paragraph, heading, list or other block-level element.
    Generic,
    /// Its elements cover too little of the visible text.
    Coverage,
    /// Its order disagrees with the pages' layout.
    Order,
}

impl ReflowRefusal {
    /// The name the JSON uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untagged => "untagged",
            Self::Empty => "empty",
            Self::Generic => "generic",
            Self::Coverage => "coverage",
            Self::Order => "order",
        }
    }
}

/// The measures behind the verdict.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ReflowChecks {
    /// Marked-content sequences in the tree.
    pub content: usize,
    /// The share of them in no block-level element.
    pub generic_share: f64,
    /// Pages whose tree gave their order.
    pub pages_used: usize,
    /// Pages whose tree the per-page checks ignored for its coverage.
    pub pages_coverage: usize,
    /// Pages whose tree the per-page checks ignored for its order.
    pub pages_order: usize,
    /// Kendall's τ between the tree's block order and the layout's, over the pages that used the
    /// tree with three blocks or more, weighted by their blocks; `None` when there are none.
    pub agreement: Option<f64>,
}

/// What a block is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(tag = "kind", rename_all = "kebab-case", rename_all_fields = "camelCase")
)]
#[non_exhaustive]
pub enum ReflowKind {
    /// A heading.
    Heading {
        /// 1 (the top) to 6.
        level: u8,
    },
    /// Running text.
    Paragraph,
    /// One item of a list.
    ListItem {
        /// The bullet or number that starts it.
        marker: String,
        /// 0 for the outermost list.
        depth: u8,
    },
    /// A paragraph of a block quote.
    Quote,
    /// A figure's or table's caption.
    Caption,
    /// A footnote or endnote.
    Note,
    /// A figure, shown as its part of the page with its alternate text.
    Figure,
    /// A formula, shown likewise.
    Formula,
    /// A table, shown likewise.
    Table,
    /// Text in no element reader mode knows.
    Other,
}

impl ReflowKind {
    /// Whether the block is shown as a crop of its page.
    #[must_use]
    pub const fn is_crop(&self) -> bool {
        matches!(self, Self::Figure | Self::Formula | Self::Table)
    }
}

/// One block of the reading view.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ReflowBlock {
    /// What it is.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: ReflowKind,
    /// The text; empty for a crop.
    pub text: String,
    /// The page it is on (a crop's first page).
    pub page: usize,
    /// Its box on that page in page space: where "back to page" goes, and a crop's clip.
    /// `None` for a crop whose content paints nothing the reader could find.
    #[cfg_attr(feature = "serde", serde(serialize_with = "ser_box"))]
    pub bbox: Option<RectPt>,
    /// A crop's alternate text (`/Alt`), when the file gives one.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub alt: Option<String>,
}

#[cfg(feature = "serde")]
#[expect(clippy::ref_option, reason = "serde's serialize_with passes the field by reference")]
fn ser_box<S: serde::Serializer>(bbox: &Option<RectPt>, serializer: S) -> Result<S::Ok, S::Error> {
    match bbox {
        Some(r) => crate::model::ser::rect(r, serializer),
        None => serializer.serialize_none(),
    }
}

impl Reflow {
    fn refused(reason: ReflowRefusal, lang: Option<String>, checks: ReflowChecks) -> Self {
        Self {
            available: false,
            reason: Some(reason),
            lang,
            blocks: Vec::new(),
            checks,
        }
    }
}

/// `doc` for reader mode, stopped by `cancel`.
///
/// # Errors
/// `cancelled`; reading errors (the catalog, the page tree, the structure tree, a page's
/// content).
pub fn reflow(doc: &Document, cancel: &CancelToken) -> Result<Reflow, ExtractError> {
    let lang = catalog_lang(doc)?;
    let tree = if doc.is_tagged()? { doc.structure()? } else { None };
    let Some(tree) = tree else {
        return Ok(Reflow::refused(ReflowRefusal::Untagged, lang, ReflowChecks::default()));
    };
    let pages = doc.pages()?;
    let page_ids: BTreeMap<PageId, usize> = (0..pages.len())
        .filter_map(|i| pages.get(i).map(|p| (p.id().clone(), i)))
        .collect();
    let facts = tree::facts(&tree, &page_ids);
    let mut checks = ReflowChecks {
        content: facts.content,
        generic_share: share(facts.generic, facts.content),
        ..ReflowChecks::default()
    };
    if facts.content == 0 {
        return Ok(Reflow::refused(ReflowRefusal::Empty, lang, checks));
    }
    if checks.generic_share > MAX_GENERIC {
        return Ok(Reflow::refused(ReflowRefusal::Generic, lang, checks));
    }
    let (extraction, feed) = run(doc, cancel)?;
    for page in &feed {
        match page.tree {
            TreeUse::Used => checks.pages_used += 1,
            TreeUse::Ignored(Implausible::Coverage) => checks.pages_coverage += 1,
            TreeUse::Ignored(Implausible::Order) => checks.pages_order += 1,
            TreeUse::None => {}
        }
    }
    checks.agreement = order::agreement(&extraction.blocks, &facts, cancel)?;
    if let Some(reason) = verdict(&checks) {
        return Ok(Reflow::refused(reason, lang, checks));
    }
    let blocks = assemble(&extraction.blocks, &facts, &feed);
    Ok(Reflow {
        available: true,
        reason: None,
        lang,
        blocks,
        checks,
    })
}

/// `part` of `whole`, 0 when `whole` is.
fn share(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    let (part, whole) = (
        u32::try_from(part).unwrap_or(u32::MAX),
        u32::try_from(whole).unwrap_or(u32::MAX),
    );
    f64::from(part) / f64::from(whole)
}

/// Why the measures refuse the document, if they do.
fn verdict(checks: &ReflowChecks) -> Option<ReflowRefusal> {
    let ignored = checks.pages_coverage + checks.pages_order;
    let tagged = checks.pages_used + ignored;
    if tagged == 0 {
        return Some(ReflowRefusal::Empty);
    }
    if share(ignored, tagged) > MAX_IGNORED {
        return Some(if checks.pages_coverage >= checks.pages_order {
            ReflowRefusal::Coverage
        } else {
            ReflowRefusal::Order
        });
    }
    match checks.agreement {
        Some(tau) if tau < MIN_AGREEMENT => Some(ReflowRefusal::Order),
        _ => None,
    }
}

/// The catalog's `/Lang`, when it is a non-empty text string (§14.9.2).
fn catalog_lang(doc: &Document) -> Result<Option<String>, ExtractError> {
    let catalog = doc.catalog()?;
    let Some(value) = catalog.get("Lang") else {
        return Ok(None);
    };
    let value = doc.deref(value)?;
    Ok(match &*value {
        laminapdf_cos::Object::String(s) => {
            Some(laminapdf_syntax::text::decode_text_string(&s.bytes)).filter(|l| !l.trim().is_empty())
        }
        _ => None,
    })
}

/// The extraction reader mode reads, with what each page gave besides its blocks.
fn run(doc: &Document, cancel: &CancelToken) -> Result<(crate::Extraction, Vec<PageFeed>), ExtractError> {
    let options = ExtractOptions::default().with_reading_order(ReadingOrder::Auto);
    let mut job = extract_job(doc, &options)?;
    job.feed = Some(Vec::new());
    job.set_cancel(cancel);
    loop {
        if let Step::Done(extraction) = job.step(doc, Budget::UNLIMITED)? {
            return Ok((extraction, job.feed.take().unwrap_or_default()));
        }
    }
}

/// A text block's kind in reader mode.
fn kind_of(block: &Block) -> ReflowKind {
    match &block.kind {
        BlockKind::Heading { level } => ReflowKind::Heading { level: *level },
        BlockKind::ListItem { marker, depth } => ReflowKind::ListItem {
            marker: marker.clone(),
            depth: *depth,
        },
        BlockKind::Caption => ReflowKind::Caption,
        BlockKind::Footnote => ReflowKind::Note,
        BlockKind::Table { .. } => ReflowKind::Table,
        BlockKind::Paragraph if block.struct_path.iter().any(|t| t == "BlockQuote") => ReflowKind::Quote,
        BlockKind::Paragraph => ReflowKind::Paragraph,
        _ => ReflowKind::Other,
    }
}

/// The object a block's element is.
fn element(block: &Block) -> Option<ObjRef> {
    block.struct_ref.as_ref().map(|r| ObjRef::new(r.obj, r.r#gen))
}

/// One entry of a page's reading order: its place in the tree's order and the block.
type Entry = (usize, ReflowBlock);

/// The blocks of reader mode: the extraction's text blocks, but those inside a crop element,
/// and the crops, each placed by its element's place in the tree.
fn assemble(blocks: &[Block], facts: &TreeFacts, feed: &[PageFeed]) -> Vec<ReflowBlock> {
    let boxes: BTreeMap<usize, &PageFeed> = feed.iter().map(|p| (p.index, p)).collect();
    // Each crop's box: what its content paints, and the text blocks it takes in.
    let mut crop_boxes: Vec<Option<(usize, RectPt)>> = facts
        .crops
        .iter()
        .map(|crop| {
            let page = crop.content.first()?.0;
            let mcids: Vec<i64> = crop
                .content
                .iter()
                .filter(|(p, _)| *p == page)
                .map(|(_, m)| *m)
                .collect();
            let painted = boxes
                .get(&page)
                .into_iter()
                .flat_map(|f| f.boxes.iter())
                .filter(|(mcid, _)| mcids.contains(mcid))
                .map(|(_, b)| *b);
            painted.reduce(|a, b| a.union(&b)).map(|b| (page, b))
        })
        .collect();
    let mut pages: BTreeMap<usize, Vec<Entry>> = BTreeMap::new();
    let mut last_order = 0;
    for block in blocks {
        let object = element(block);
        let crop = object.and_then(|o| facts.in_crop.get(&o)).copied();
        if let Some(crop) = crop.filter(|_| block.kind != BlockKind::Caption) {
            if let Some(slot) = crop_boxes.get_mut(crop) {
                *slot = match *slot {
                    Some((page, bbox)) if page == block.page => Some((page, bbox.union(&block.bbox))),
                    Some(other) => Some(other),
                    None => Some((block.page, block.bbox)),
                };
            }
            continue;
        }
        // Text outside the tree follows the block before it.
        let order = object.and_then(|o| facts.order.get(&o)).copied().unwrap_or(last_order);
        last_order = order;
        pages.entry(block.page).or_default().push((
            order,
            ReflowBlock {
                kind: kind_of(block),
                text: block.text.clone(),
                page: block.page,
                bbox: Some(block.bbox),
                alt: None,
            },
        ));
    }
    for (crop, place) in facts.crops.iter().zip(crop_boxes) {
        let Some(page) = place.map(|(p, _)| p).or_else(|| crop.content.first().map(|(p, _)| *p)) else {
            continue;
        };
        let kind = match crop.kind {
            CropKind::Figure => ReflowKind::Figure,
            CropKind::Formula => ReflowKind::Formula,
            CropKind::Table => ReflowKind::Table,
        };
        pages.entry(page).or_default().push((
            crop.order,
            ReflowBlock {
                kind,
                text: String::new(),
                page,
                bbox: place.map(|(_, b)| b),
                alt: crop.alt.clone(),
            },
        ));
    }
    pages
        .into_values()
        .flat_map(|mut entries| {
            // Stable: blocks of one element keep extraction's order.
            entries.sort_by_key(|(order, _)| *order);
            entries.into_iter().map(|(_, b)| b)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ReflowChecks, ReflowRefusal, verdict};

    #[test]
    fn the_verdict_refuses_ignored_pages_by_their_main_reason_then_a_low_agreement() {
        let checks = |used, coverage, order, agreement| ReflowChecks {
            pages_used: used,
            pages_coverage: coverage,
            pages_order: order,
            agreement,
            ..ReflowChecks::default()
        };
        assert_eq!(verdict(&checks(0, 0, 0, None)), Some(ReflowRefusal::Empty));
        assert_eq!(verdict(&checks(1, 2, 1, Some(1.0))), Some(ReflowRefusal::Coverage));
        assert_eq!(verdict(&checks(1, 0, 2, Some(1.0))), Some(ReflowRefusal::Order));
        assert_eq!(verdict(&checks(4, 1, 0, Some(1.0))), None);
        assert_eq!(verdict(&checks(4, 0, 0, Some(-0.5))), Some(ReflowRefusal::Order));
        assert_eq!(verdict(&checks(4, 0, 0, None)), None);
    }
}
