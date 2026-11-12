//! Reader mode: a document as reflowable blocks, in its structure tree's order when the tree can
//! be trusted (tier 1, below), else in the order extraction reads from the page layout, labelled
//! approximate (tier 2, [`approximate`]).
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
//! - turns to the layout (tier 2) for a document whose tree is missing or empty, whose content sits mostly in no
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

mod approximate;
mod order;
mod tree;

use std::collections::BTreeMap;

use laminapdf_cos::ObjRef;
use laminapdf_model::{Document, PageId, RectPt};
use laminapdf_syntax::{Budget, CancelToken, Step};

use crate::error::ExtractError;
use crate::job::{PageFeed, TreeSource, extract_job_with};
use crate::model::{Block, BlockKind};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::page::TreeUse;
use crate::tagged::{Implausible, TaggedIndex};
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
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct Reflow {
    /// Whether reader mode can show the document; when it can't, `blocks` is empty. Only a
    /// document with nothing to show, no text and no picture on any page, can't be shown.
    pub available: bool,
    /// Where the blocks' order comes from.
    pub source: ReflowSource,
    /// Why the structure tree wasn't used: set whenever `source` is
    /// [`ReflowSource::Approximate`] (an untagged file is [`ReflowRefusal::Untagged`]).
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

/// Where reader mode's order comes from.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum ReflowSource {
    /// The document's structure tree, which passed every check.
    Tags,
    /// The page layout, as extraction reads it: headings, lists and the order of columns are
    /// guessed, so the reading view says so.
    Approximate,
}

/// Why reader mode doesn't use a document's structure tree.
///
/// Added in 0.2.0.
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
    /// No page has text or a picture to show.
    Nothing,
}

impl ReflowRefusal {
    /// The name the JSON uses.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Untagged => "untagged",
            Self::Empty => "empty",
            Self::Generic => "generic",
            Self::Coverage => "coverage",
            Self::Order => "order",
            Self::Nothing => "nothing",
        }
    }
}

/// The measures behind the verdict.
///
/// Added in 0.2.0.
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
    /// Pages the approximate reading view shows as their image (scanned, or read with too
    /// little confidence); 0 with tags.
    pub page_images: usize,
}

/// What a block is.
///
/// Added in 0.2.0.
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
    /// A whole page shown as its image, in the approximate reading view only: its `text` is the
    /// page's text in layout order, for searching, but the view doesn't reflow it.
    PageImage {
        /// Why the page isn't reflowed.
        why: PageImageReason,
    },
    /// Text in no element reader mode knows.
    Other,
}

impl ReflowKind {
    /// Whether the block is shown as a crop of its page.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_crop(&self) -> bool {
        matches!(
            self,
            Self::Figure | Self::Formula | Self::Table | Self::PageImage { .. }
        )
    }
}

/// Why the approximate reading view shows a page as its image.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[non_exhaustive]
pub enum PageImageReason {
    /// The page is a picture of text (a scan): no visible text, or an OCR layer under an image.
    Scanned,
    /// Extraction doubted the order of too much of its text (columns too close, or a grid).
    Uncertain,
}

/// One block of the reading view.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[non_exhaustive]
pub struct ReflowBlock {
    /// What it is.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub kind: ReflowKind,
    /// The text; empty for a crop, but for an approximate table's or page image's, which
    /// keeps the text extraction read there (for searching; the view shows the crop).
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

/// `doc` for reader mode, stopped by `cancel`.
///
/// # Errors
/// `cancelled`; reading errors (the catalog, the page tree, the structure tree, a page's
/// content).
///
/// Added in 0.2.0.
pub fn reflow(doc: &Document, cancel: &CancelToken) -> Result<Reflow, ExtractError> {
    let lang = catalog_lang(doc)?;
    let tree = if doc.is_tagged()? { doc.structure()? } else { None };
    let Some(tree) = tree else {
        return approximate::reflow(doc, ReflowRefusal::Untagged, lang, ReflowChecks::default(), cancel);
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
        return approximate::reflow(doc, ReflowRefusal::Empty, lang, checks, cancel);
    }
    if checks.generic_share > MAX_GENERIC {
        return approximate::reflow(doc, ReflowRefusal::Generic, lang, checks, cancel);
    }
    let index = TaggedIndex::from_tree(doc, &tree)?;
    let options = ExtractOptions::default().with_reading_order(ReadingOrder::Auto);
    let (extraction, feed) = run(doc, &options, TreeSource::Given(index), cancel)?;
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
        return approximate::reflow(doc, reason, lang, checks, cancel);
    }
    let blocks = assemble(&extraction.blocks, &facts, &feed);
    Ok(Reflow {
        available: true,
        source: ReflowSource::Tags,
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

/// The extraction reader mode reads, with `options` and the tree from `tree`, with what each page
/// gave besides its blocks.
fn run(
    doc: &Document,
    options: &ExtractOptions,
    tree: TreeSource,
    cancel: &CancelToken,
) -> Result<(crate::Extraction, Vec<PageFeed>), ExtractError> {
    let mut job = extract_job_with(doc, options, tree)?;
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

/// One entry of a page's reading order: its element's place in the tree's order (`None` for
/// text in no element) and the block.
type Entry = (Option<usize>, ReflowBlock);

/// A page's entries in reading order. Text in no element follows the text before it on the page.
/// On a page with no text from the tree (one whose tree the per-page checks ignored, so its text
/// is in the layout's order), each crop goes before the first text below its top.
fn page_order(entries: Vec<Entry>) -> Vec<ReflowBlock> {
    let (text, crops): (Vec<Entry>, Vec<Entry>) = entries.into_iter().partition(|(_, b)| !b.kind.is_crop());
    if text.iter().all(|(order, _)| order.is_none()) {
        let top = |b: &ReflowBlock| b.bbox.map_or(f64::INFINITY, |r| r.y0.get());
        let mut crops: Vec<ReflowBlock> = crops.into_iter().map(|(_, b)| b).collect();
        crops.sort_by(|a, b| top(a).total_cmp(&top(b)));
        let mut out = Vec::new();
        let mut crops = crops.into_iter().peekable();
        for (_, block) in text {
            while let Some(crop) = crops.next_if(|c| top(c) < top(&block)) {
                out.push(crop);
            }
            out.push(block);
        }
        out.extend(crops);
        return out;
    }
    let mut last = 0;
    let mut keyed: Vec<(usize, ReflowBlock)> = text
        .into_iter()
        .map(|(order, block)| {
            last = order.unwrap_or(last);
            (last, block)
        })
        .chain(crops.into_iter().map(|(order, block)| (order.unwrap_or(0), block)))
        .collect();
    // Stable: blocks of one element keep extraction's order, and text before crops of equal place.
    keyed.sort_by_key(|(order, _)| *order);
    keyed.into_iter().map(|(_, b)| b).collect()
}

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
        pages.entry(block.page).or_default().push((
            object.and_then(|o| facts.order.get(&o)).copied(),
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
            Some(crop.order),
            ReflowBlock {
                kind,
                text: String::new(),
                page,
                bbox: place.map(|(_, b)| b),
                alt: crop.alt.clone(),
            },
        ));
    }
    pages.into_values().flat_map(page_order).collect()
}

#[cfg(test)]
mod tests {
    use laminapdf_model::RectPt;

    use super::{ReflowBlock, ReflowChecks, ReflowKind, ReflowRefusal, page_order, verdict};

    fn block(kind: ReflowKind, text: &str, y: f64) -> ReflowBlock {
        ReflowBlock {
            kind,
            text: text.to_owned(),
            page: 0,
            bbox: Some(RectPt::from_points(50.0, y, 100.0, 20.0)),
            alt: None,
        }
    }

    fn texts(blocks: &[ReflowBlock]) -> Vec<&str> {
        blocks.iter().map(|b| b.text.as_str()).collect()
    }

    #[test]
    fn untagged_text_follows_the_text_before_it_and_crops_take_their_place_in_the_tree() {
        let got = page_order(vec![
            (Some(5), block(ReflowKind::Paragraph, "a", 100.0)),
            (None, block(ReflowKind::Paragraph, "b", 400.0)),
            (Some(9), block(ReflowKind::Paragraph, "c", 200.0)),
            (Some(7), block(ReflowKind::Figure, "fig", 300.0)),
        ]);
        assert_eq!(texts(&got), ["a", "b", "fig", "c"]);
    }

    #[test]
    fn on_a_page_with_no_tree_text_crops_go_in_by_their_height() {
        let got = page_order(vec![
            (None, block(ReflowKind::Paragraph, "top", 100.0)),
            (None, block(ReflowKind::Paragraph, "bottom", 500.0)),
            (Some(3), block(ReflowKind::Figure, "fig", 300.0)),
        ]);
        assert_eq!(texts(&got), ["top", "fig", "bottom"]);
    }

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
