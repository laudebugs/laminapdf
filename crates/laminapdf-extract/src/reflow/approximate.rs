//! Tier 2: reader mode from the page layout, for a document with no structure tree reader mode
//! can trust.
//!
//! The blocks are extraction's in [`ReadingOrder::Layout`]: XY-Cut++ columns, headings by size
//! and weight, list items by their markers, ruled tables, running heads and feet dropped by
//! repetition (ADR 0030 §2–§4). On top of them, per page:
//!
//! - a page that is a picture of text (no visible text but images, or text only in an OCR layer:
//!   [`TextLayerStatus`]) is shown as its image, since there is no text to reflow until OCR; an
//!   OCR layer's text, invisible and so dropped from the blocks, is kept for searching;
//! - a page whose text extraction read with low confidence (columns too close to call, or a grid
//!   read row by row: the column-order checks' blocks at 0.5) is shown as its image too, with its
//!   text kept for searching, rather than reflowed in an order that may be wrong;
//! - tables are crops of their box, like the tagged view's;
//! - images big enough to be figures are crops (images that overlap or touch joined first, so a
//!   picture drawn in tiles is one figure), each placed before the first text below it in its
//!   column; text wholly inside a figure (a chart's labels) goes into it, kept for searching; and a
//!   short paragraph that starts "Figure", "Fig." or "Table" next to a crop is its caption.
//!
//! No layout model runs here (ADR 0030 §6): these are rules over extraction's output, so the view
//! is labelled approximate.

use std::collections::BTreeMap;

use laminapdf_model::{Document, RectPt};
use laminapdf_syntax::CancelToken;

use super::{
    PageImageReason, Reflow, ReflowBlock, ReflowChecks, ReflowKind, ReflowRefusal, ReflowSource, kind_of, run,
};
use crate::error::ExtractError;
use crate::job::TreeSource;
use crate::model::{Block, BlockKind, DropReason, Dropped, ExtractedPage, TextLayerStatus};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::regions::{area, inside};

/// The least confidence a page's text may have, by characters, before the page is shown as its
/// image. Layout blocks score 0.75 and doubted ones 0.5, so this keeps a page with up to about
/// two fifths of its text doubted.
const MIN_PAGE_CONFIDENCE: f64 = 0.65;

/// The shortest side, in points, of an image shown as a figure (half an inch): smaller ones are
/// icons, rules and bullets.
const MIN_FIGURE_SIDE: f64 = 36.0;

/// The share of its page an image covers to be the page itself (a scan or a background) rather
/// than a figure.
const PAGE_COVER: f64 = 0.85;

/// How far, in points, a caption may sit from its crop's top or bottom edge.
const CAPTION_GAP: f64 = 24.0;

/// The longest caption, in characters.
const CAPTION_CHARS: usize = 300;

/// How far, in points, text may stick out of a figure and still be the figure's.
const LABEL_SLACK: f64 = 2.0;

/// `doc` from its layout, the tree unused for `reason`, with the tier-1 `checks` measured so far.
///
/// # Errors
/// `cancelled`; reading errors (the page tree, a page's content).
pub(super) fn reflow(
    doc: &Document,
    reason: ReflowRefusal,
    lang: Option<String>,
    mut checks: ReflowChecks,
    cancel: &CancelToken,
) -> Result<Reflow, ExtractError> {
    // An OCR layer's text is invisible, so extraction drops it; its spans keep it for searching.
    let mut options = ExtractOptions::default().with_reading_order(ReadingOrder::Layout);
    options.drop.keep_spans = true;
    let (extraction, feed) = run(doc, &options, TreeSource::Given(None), cancel)?;
    let mut by_page: BTreeMap<usize, Vec<&Block>> = BTreeMap::new();
    for block in &extraction.blocks {
        by_page.entry(block.page).or_default().push(block);
    }
    let mut blocks = Vec::new();
    for page in &extraction.pages {
        let text = by_page.get(&page.index).map_or(&[][..], Vec::as_slice);
        // A scan of the feed rather than a map: a sort's code for one lookup per page costs the
        // WASM module more than the scan costs the reader.
        let images = feed
            .iter()
            .find(|f| f.index == page.index)
            .map_or(&[][..], |f| f.images.as_slice());
        let shown = page_blocks(page, text, images, &extraction.dropped);
        checks.page_images += shown
            .iter()
            .filter(|b| matches!(b.kind, ReflowKind::PageImage { .. }))
            .count();
        blocks.extend(shown);
    }
    let available = !blocks.is_empty();
    Ok(Reflow {
        available,
        source: ReflowSource::Approximate,
        reason: Some(if available { reason } else { ReflowRefusal::Nothing }),
        lang,
        blocks,
        checks,
    })
}

/// One page's blocks in reading order: its image, or its text with its figures and tables.
/// `dropped` is the document's dropped text, whose invisible runs on a scan are its OCR layer.
fn page_blocks(page: &ExtractedPage, text: &[&Block], images: &[RectPt], dropped: &[Dropped]) -> Vec<ReflowBlock> {
    let whole = RectPt::from_points(0.0, 0.0, page.width.get(), page.height.get());
    let covers_page = |b: &RectPt| area(b) >= PAGE_COVER * area(&whole);
    let scanned = matches!(
        page.text_layer.status,
        TextLayerStatus::NeedsOcr | TextLayerStatus::OcrLayer
    ) || (text.is_empty() && images.iter().any(covers_page));
    if scanned || (!text.is_empty() && confidence(text) < MIN_PAGE_CONFIDENCE) {
        let why = if scanned {
            PageImageReason::Scanned
        } else {
            PageImageReason::Uncertain
        };
        let mut words: Vec<&str> = text.iter().map(|b| b.text.as_str()).filter(|t| !t.is_empty()).collect();
        let separator = if words.is_empty() {
            // Only a scan with no visible text gets here, so the filter runs on few pages.
            words = dropped
                .iter()
                .filter(|d| d.page == page.index && d.reason == DropReason::Invisible)
                .flat_map(|d| d.spans.iter().map(|s| s.text.as_str()))
                .collect();
            " "
        } else {
            "\n\n"
        };
        return vec![ReflowBlock {
            kind: ReflowKind::PageImage { why },
            text: words.join(separator),
            page: page.index,
            bbox: Some(whole),
            alt: None,
        }];
    }
    let mut out: Vec<ReflowBlock> = text.iter().map(|b| text_block(b)).collect();
    // Tiles of one picture make one image first; only then are small and page-sized ones left out.
    let mut figures: Vec<RectPt> = merged(images.to_vec())
        .into_iter()
        .filter(|b| !covers_page(b) && b.width().get() >= MIN_FIGURE_SIDE && b.height().get() >= MIN_FIGURE_SIDE)
        .collect();
    // Images under a table's box are the table's.
    figures.retain(|f| {
        !out.iter()
            .any(|b| b.kind == ReflowKind::Table && b.bbox.is_some_and(|t| inside(&t, f)))
    });
    for figure in figures {
        let labels = take_labels(&mut out, &figure);
        let at = place(&out, &figure);
        out.insert(
            at,
            ReflowBlock {
                kind: ReflowKind::Figure,
                text: labels,
                page: page.index,
                bbox: Some(figure),
                alt: None,
            },
        );
    }
    mark_captions(&mut out);
    out
}

/// Takes the paragraphs wholly inside `figure` (within [`LABEL_SLACK`]) out of `blocks`, and
/// returns their text: a chart's labels belong to the figure, not to the column.
fn take_labels(blocks: &mut Vec<ReflowBlock>, figure: &RectPt) -> String {
    let within = |b: &RectPt| {
        b.x0.get() >= figure.x0.get() - LABEL_SLACK
            && b.y0.get() >= figure.y0.get() - LABEL_SLACK
            && b.x1.get() <= figure.x1.get() + LABEL_SLACK
            && b.y1.get() <= figure.y1.get() + LABEL_SLACK
    };
    let mut labels = Vec::new();
    blocks.retain(|b| {
        let take = matches!(b.kind, ReflowKind::Paragraph | ReflowKind::Other) && b.bbox.is_some_and(|r| within(&r));
        if take {
            labels.push(b.text.clone());
        }
        !take
    });
    labels.join(" ")
}

/// A layout block for the reading view: tables become crops that keep their text.
fn text_block(block: &Block) -> ReflowBlock {
    ReflowBlock {
        kind: kind_of(block),
        text: block.text.clone(),
        page: block.page,
        bbox: Some(block.bbox),
        alt: None,
    }
}

/// The page's text confidence: the blocks' mean, weighted by their characters. Tables don't
/// count: their confidence is the grid's, not the order's.
fn confidence(text: &[&Block]) -> f64 {
    let (mut sum, mut chars) = (0.0, 0.0);
    for block in text.iter().filter(|b| !matches!(b.kind, BlockKind::Table { .. })) {
        let n = f64::from(u32::try_from(block.text.chars().count()).unwrap_or(u32::MAX));
        sum += block.confidence * n;
        chars += n;
    }
    if chars == 0.0 { 1.0 } else { sum / chars }
}

/// Where a figure goes among the page's blocks: before the first block in reading order that
/// starts below the figure's top and shares some of its width (its column), else after the last
/// block above it in its column, else by height alone.
fn place(blocks: &[ReflowBlock], figure: &RectPt) -> usize {
    let column = |b: &RectPt| b.x0.get() < figure.x1.get() && figure.x0.get() < b.x1.get();
    let boxes: Vec<Option<RectPt>> = blocks.iter().map(|b| b.bbox).collect();
    if let Some(i) = boxes
        .iter()
        .position(|b| b.is_some_and(|b| column(&b) && b.y0.get() >= figure.y0.get()))
    {
        return i;
    }
    if let Some(i) = boxes.iter().rposition(|b| b.is_some_and(|b| column(&b))) {
        return i + 1;
    }
    boxes
        .iter()
        .position(|b| b.is_some_and(|b| b.y0.get() >= figure.y0.get()))
        .unwrap_or(blocks.len())
}

/// Gives a crop's caption its kind: a short paragraph next to the crop's top or bottom edge that
/// starts with "Figure", "Fig." or "Table", moved after its crop.
fn mark_captions(blocks: &mut Vec<ReflowBlock>) {
    let mut i = 0;
    while i < blocks.len() {
        let is_caption = blocks
            .get(i)
            .is_some_and(|b| b.kind == ReflowKind::Paragraph && names_a_crop(&b.text));
        if is_caption {
            let near = blocks.get(i).and_then(|b| b.bbox).and_then(|cap| {
                blocks.iter().position(|c| {
                    matches!(c.kind, ReflowKind::Figure | ReflowKind::Table)
                        && c.bbox.is_some_and(|crop| beside(&crop, &cap))
                })
            });
            if let (Some(crop), Some(block)) = (near, blocks.get_mut(i)) {
                block.kind = ReflowKind::Caption;
                if crop > i {
                    let caption = blocks.remove(i);
                    blocks.insert(crop, caption);
                    continue;
                }
            }
        }
        i += 1;
    }
}

/// Whether `text` reads like a figure's or table's caption.
fn names_a_crop(text: &str) -> bool {
    let text = text.trim_start();
    text.chars().count() <= CAPTION_CHARS
        && ["Figure", "Fig.", "Table"].iter().any(|w| {
            text.strip_prefix(w)
                .is_some_and(|rest| rest.starts_with([' ', '\u{a0}']))
        })
}

/// Whether `caption` sits just above or below `crop`, sharing some of its width.
fn beside(crop: &RectPt, caption: &RectPt) -> bool {
    let shares = caption.x0.get() < crop.x1.get() && crop.x0.get() < caption.x1.get();
    let below = (caption.y0.get() - crop.y1.get()).abs() <= CAPTION_GAP;
    let above = (crop.y0.get() - caption.y1.get()).abs() <= CAPTION_GAP;
    shares && (below || above)
}

/// Whether `a` and `b` overlap or share an edge.
fn touches(a: &RectPt, b: &RectPt) -> bool {
    a.x0 <= b.x1 && b.x0 <= a.x1 && a.y0 <= b.y1 && b.y0 <= a.y1
}

/// `boxes` with every pair that overlaps or touches joined, so tiles of one picture make one figure.
fn merged(mut boxes: Vec<RectPt>) -> Vec<RectPt> {
    let mut out: Vec<RectPt> = Vec::new();
    while let Some(mut b) = boxes.pop() {
        while let Some(i) = out.iter().position(|o| touches(o, &b)) {
            b = b.union(&out.swap_remove(i));
        }
        out.push(b);
    }
    // In no order: `place` puts each figure by its position, whatever order they come in.
    out
}

#[cfg(test)]
mod tests {
    use laminapdf_model::RectPt;

    use super::{ReflowBlock, ReflowKind, mark_captions, merged, names_a_crop, place, take_labels};

    fn block(kind: ReflowKind, text: &str, x: f64, y: f64, w: f64, h: f64) -> ReflowBlock {
        ReflowBlock {
            kind,
            text: text.to_owned(),
            page: 0,
            bbox: Some(RectPt::from_points(x, y, w, h)),
            alt: None,
        }
    }

    #[test]
    fn a_figure_goes_before_the_first_text_below_it_in_its_column() {
        // Two columns: left 50–250, right 300–500; the figure is in the right column at 200.
        let blocks = [
            block(ReflowKind::Paragraph, "left top", 50.0, 100.0, 200.0, 80.0),
            block(ReflowKind::Paragraph, "left bottom", 50.0, 400.0, 200.0, 80.0),
            block(ReflowKind::Paragraph, "right top", 300.0, 100.0, 200.0, 80.0),
            block(ReflowKind::Paragraph, "right bottom", 300.0, 400.0, 200.0, 80.0),
        ];
        assert_eq!(place(&blocks, &RectPt::from_points(300.0, 200.0, 200.0, 150.0)), 3);
        // Below everything in its column: after that column's last block.
        assert_eq!(place(&blocks, &RectPt::from_points(50.0, 600.0, 200.0, 100.0)), 2);
    }

    #[test]
    fn a_caption_beside_a_crop_takes_its_kind_and_follows_it() {
        let mut blocks = vec![
            block(ReflowKind::Paragraph, "Figure 1. The orchard", 50.0, 80.0, 200.0, 14.0),
            block(ReflowKind::Figure, "", 50.0, 100.0, 200.0, 150.0),
            block(
                ReflowKind::Paragraph,
                "Figures are hard to place.",
                50.0,
                300.0,
                200.0,
                40.0,
            ),
        ];
        mark_captions(&mut blocks);
        let kinds: Vec<&ReflowKind> = blocks.iter().map(|b| &b.kind).collect();
        assert_eq!(
            kinds,
            [&ReflowKind::Figure, &ReflowKind::Caption, &ReflowKind::Paragraph]
        );
    }

    #[test]
    fn captions_name_a_figure_or_table_with_a_word_and_a_space() {
        assert!(names_a_crop("Figure 2: Yields"));
        assert!(names_a_crop("Table 4 — Costs"));
        assert!(names_a_crop("Fig. 3"));
        assert!(!names_a_crop("Figures show the trend."));
        assert!(!names_a_crop("Tablet sales rose."));
    }

    #[test]
    fn overlapping_image_boxes_make_one_figure() {
        let mut got = merged(vec![
            RectPt::from_points(0.0, 0.0, 100.0, 100.0),
            RectPt::from_points(90.0, 0.0, 100.0, 100.0),
            RectPt::from_points(0.0, 300.0, 50.0, 50.0),
        ]);
        got.sort_by(|a, b| a.y0.get().total_cmp(&b.y0.get()));
        assert_eq!(
            got,
            [
                RectPt::from_points(0.0, 0.0, 190.0, 100.0),
                RectPt::from_points(0.0, 300.0, 50.0, 50.0)
            ]
        );
    }

    #[test]
    fn image_boxes_that_only_share_an_edge_make_one_figure() {
        let got = merged(vec![
            RectPt::from_points(0.0, 0.0, 100.0, 100.0),
            RectPt::from_points(100.0, 0.0, 100.0, 100.0),
        ]);
        assert_eq!(got, [RectPt::from_points(0.0, 0.0, 200.0, 100.0)]);
    }

    #[test]
    fn text_inside_a_figure_moves_into_it_and_text_outside_stays() {
        let mut blocks = vec![
            block(ReflowKind::Paragraph, "2019", 60.0, 120.0, 30.0, 10.0),
            block(ReflowKind::Paragraph, "Yield", 60.0, 200.0, 30.0, 10.0),
            block(ReflowKind::Paragraph, "Body text.", 50.0, 300.0, 200.0, 40.0),
        ];
        let labels = take_labels(&mut blocks, &RectPt::from_points(50.0, 100.0, 200.0, 150.0));
        let left: Vec<&str> = blocks.iter().map(|b| b.text.as_str()).collect();
        assert_eq!((labels.as_str(), left), ("2019 Yield", vec!["Body text."]));
    }
}
