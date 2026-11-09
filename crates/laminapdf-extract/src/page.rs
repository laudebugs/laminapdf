//! One page: its runs flagged or kept, kept runs grouped into lines and blocks, the blocks put
//! in reading order.

use laminapdf_cos::Warning;
use laminapdf_model::RectPt;
use laminapdf_model::{Document, Page, Pt};
use laminapdf_syntax::CancelToken;
use laminapdf_text::PageText;

use crate::blocks::{Draft, blocks};
use crate::error::ExtractError;
use crate::lines::{Line, Piece, lines, mostly_rtl};
use crate::marks::Drawn;
use crate::model::{Block, BlockKind, Coordinates, DropReason, Dropped, ExtractedPage, Flags, Span, TextLayerStatus};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::plugins::{LayoutRegion, OcrWord, RegionLabel};
use crate::regions::{inside, kind_for, ocr_pieces, region_near, region_of};
use crate::segments::{Covers, segments};
use crate::tables::{self, Edges, Glyph, Table, TableMode, is_bold};
use crate::tagged::{self, Implausible, TaggedIndex, TreeBlocks};
use crate::text_fix::{decompose_ligatures, join_lines, visible_chars};
use crate::text_layer::measure;
use crate::visibility::{Backdrops, Look, flags, normalised};
use crate::xycut;

/// What one page gave.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageOut {
    pub(crate) info: ExtractedPage,
    pub(crate) blocks: Vec<Block>,
    pub(crate) dropped: Vec<Dropped>,
    /// How the page used the structure tree.
    pub(crate) tree: TreeUse,
}

/// How a page used the structure tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeUse {
    /// The tree gave its order.
    Used,
    /// It had tagged content, but the tree failed a check.
    Ignored(Implausible),
    /// No tree, or nothing of the tree on this page.
    None,
}

/// How sure the layout rules are of a block (ADR 0030 §1: below the structure tree's 1).
const LAYOUT_CONFIDENCE: f64 = 0.75;

/// Content order is the drawing order, which is often not the reading order.
const CONTENT_CONFIDENCE: f64 = 0.5;

/// A text block whose column order the layout rules had to guess: as sure as content order.
const UNCERTAIN_CONFIDENCE: f64 = 0.5;

/// A block where this many lines, and at least half its lines, hold narrow-gutter points within
/// half a font size of one another has probably been read across a gutter narrower than one
/// font size; loose word spaces line up on a few lines of a long paragraph by chance.
const NARROW_GUTTER_LINES: usize = 3;

/// Raises `extract-column-order-uncertain` when the page's column order is in doubt, and returns
/// for each item whether a doubt covers it. Only where the layout rules decided the whole page's
/// order (`whole_page`: not the text a structure tree left out): not in content order, with a
/// layout model's regions, or on a page with vertical writing (which
/// `extract-reading-order-ambiguous` covers).
fn report_doubts(
    cx: &PageContext<'_>,
    items: &[(Item, Option<usize>)],
    lines: &[Line],
    doubt: Option<xycut::Doubt>,
    whole_page: bool,
    vertical: bool,
) -> Result<Vec<bool>, ExtractError> {
    let layout_order = whole_page
        && cx.options.reading_order != ReadingOrder::ContentStream
        && cx.painting.regions.is_empty()
        && !vertical;
    if !layout_order {
        return Ok(vec![false; items.len()]);
    }
    let (reason, uncertain) = column_doubts(items, lines, doubt);
    if let Some(reason) = reason {
        cx.doc.report(Warning::ExtractColumnOrderUncertain {
            page: u64::try_from(cx.info.index).unwrap_or(u64::MAX),
            reason,
        })?;
    }
    let mut doubted = vec![false; items.len()];
    for i in uncertain {
        if let Some(d) = doubted.get_mut(i) {
            *d = true;
        }
    }
    Ok(doubted)
}

/// The page's doubts about its column order: the first reason found (`narrow-gutter`, then
/// XY-Cut++'s `columns-revisited` or `near-grid`) and every item (index) a doubt covers.
fn column_doubts(
    items: &[(Item, Option<usize>)],
    lines: &[Line],
    doubt: Option<xycut::Doubt>,
) -> (Option<&'static str>, Vec<usize>) {
    let mut uncertain: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, (item, _))| matches!(item, Item::Text(d) if read_across_gutter(d, lines)))
        .map(|(i, _)| i)
        .collect();
    let mut reason = (!uncertain.is_empty()).then_some("narrow-gutter");
    if let Some(doubt) = doubt {
        reason = reason.or(Some(doubt.reason));
        uncertain.extend(doubt.blocks);
    }
    uncertain.sort_unstable();
    uncertain.dedup();
    (reason, uncertain)
}

/// Whether at least [`NARROW_GUTTER_LINES`] of `draft`'s lines, and at least half of them, kept
/// runs together across a gap that may be a narrow gutter, at points within half a font size of
/// one another.
fn read_across_gutter(draft: &Draft, lines: &[Line]) -> bool {
    let points: Vec<(usize, f64, f64)> = draft
        .lines
        .iter()
        .filter_map(|&l| lines.get(l).map(|line| (l, line)))
        .flat_map(|(l, line)| line.near_gutters.iter().map(move |&x| (l, x, line.size.get())))
        .collect();
    points.iter().any(|&(_, x, size)| {
        let mut near: Vec<usize> = points
            .iter()
            .filter(|&&(_, other, _)| other >= x && other - x <= size / 2.0)
            .map(|&(l, _, _)| l)
            .collect();
        near.sort_unstable();
        near.dedup();
        near.len() >= NARROW_GUTTER_LINES.max(draft.lines.len().div_ceil(2))
    })
}

/// A dropped entry for `span`'s text, with the span when the options keep spans.
fn dropped(index: usize, reason: DropReason, bbox: RectPt, span: &Span, options: &ExtractOptions) -> Dropped {
    Dropped {
        reason,
        page: index,
        bbox,
        chars: visible_chars(&span.text),
        spans: if options.drop.keep_spans {
            vec![span.clone()]
        } else {
            Vec::new()
        },
    }
}

/// The page's runs as spans: kept ones (with their pieces) and dropped ones, and the boxes of
/// the covered text, which never opens a gutter in a line.
struct Runs {
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    dropped: Vec<Dropped>,
    covered: Vec<RectPt>,
}

/// What extraction reads of a page besides its text: what it painted, seen in the same walk of
/// its content, and the document's structure tree.
pub(crate) struct PageInput<'a> {
    /// The table rules, when tables are looked for.
    pub(crate) edges: Option<&'a Edges>,
    /// What text can stand on, for the low-contrast flag.
    pub(crate) backdrops: &'a Backdrops,
    /// What is painted over earlier text, for the covered flag.
    pub(crate) covers: &'a Covers,
    /// The images and paths drawn, for the text-layer status.
    pub(crate) drawn: &'a Drawn,
    /// The structure tree, when the reading order may come from it.
    pub(crate) tree: Option<&'a TaggedIndex>,
    /// The layout model's regions on the page; empty without one.
    pub(crate) regions: &'a [LayoutRegion],
    /// The OCR engine's words on the page; empty unless it read the page.
    pub(crate) words: &'a [OcrWord],
}

/// The hidden runs of page `index` (§4's flags), artifacts left out: what [`crate::hidden_text`]
/// lists.
pub(crate) fn hidden_runs(
    index: usize,
    text: &PageText,
    painting: &PageInput<'_>,
    options: &ExtractOptions,
) -> Vec<Dropped> {
    let mut dropped = runs(index, text, painting, options).dropped;
    dropped.retain(|d| d.reason != DropReason::Artifact);
    dropped
}

fn runs(index: usize, text: &PageText, painting: &PageInput<'_>, options: &ExtractOptions) -> Runs {
    let (backdrops, regions) = (painting.backdrops, painting.regions);
    let frame = &text.frame;
    let (width, height, unit) = (frame.width(), frame.height(), frame.user_unit());
    let mut out = Runs {
        pieces: Vec::new(),
        spans: Vec::new(),
        dropped: Vec::new(),
        covered: Vec::new(),
    };
    for run in &text.runs {
        let size = Pt::new(run.font_size * unit);
        let baseline = text
            .glyphs
            .get(run.glyphs.start)
            .map_or(run.bounds.y1, |g| frame.to_page(g.origin).y);
        let span_flags = flags(&Look::of(run), &run.bounds, size, width, height, backdrops);
        let span = Span {
            text: run.text.clone(),
            bbox: run.bounds,
            baseline,
            font: run.font_name.to_string(),
            size,
            flags: span_flags,
            ocr: None,
        };
        // Artifacts are furniture by the file's own word (ISO 32000-2 §14.8.2.2).
        let artifact = (run.artifact && options.drop.furniture).then_some(DropReason::Artifact);
        if let Some(reason) = span_flags.reasons().first().copied().or(artifact) {
            out.dropped.push(dropped(index, reason, run.bounds, &span, options));
            continue;
        }
        let cut = segments(text, run, size, regions, painting.covers);
        for (words, bbox) in cut.covered {
            let flags = Flags {
                covered: true,
                ..Flags::NONE
            };
            let hidden = Span {
                text: words,
                bbox,
                flags,
                ..span.clone()
            };
            out.dropped
                .push(dropped(index, DropReason::Covered, bbox, &hidden, options));
            out.covered.push(bbox);
        }
        for (words, bbox, near_gutters) in cut.kept {
            let words = if run.actual_text {
                words
            } else {
                decompose_ligatures(&words)
            };
            if words.trim().is_empty() {
                continue;
            }
            out.pieces.push(Piece {
                text: words.clone(),
                bbox,
                baseline,
                size,
                font: span.font.clone(),
                vertical: run.vertical,
                mcid: run.mcid,
                artifact: run.artifact,
                near_gutters,
            });
            out.spans.push(Span {
                text: words,
                bbox,
                ..span.clone()
            });
        }
    }
    out
}

fn is_vertical(line: &Line, pieces: &[Piece]) -> bool {
    line.pieces
        .first()
        .and_then(|&i| pieces.get(i))
        .is_some_and(|p| p.vertical)
}

/// A block to be: text grouped into lines, or a table (an index into the page's tables).
enum Item {
    Text(Draft),
    Table(usize),
}

/// The order to read items in, as indices. `first` is each item's first run in drawing order
/// (for [`ReadingOrder::ContentStream`]), `lines` its line count (a table counts its rows).
/// With a layout model's regions (`region`, per item), XY-Cut++ orders the regions and the
/// items left outside any, and a region's own items read top to bottom.
fn order_of(
    boxes: &[RectPt],
    lines: &[usize],
    first: &[usize],
    region: &[Option<usize>],
    rtl: bool,
    options: &ExtractOptions,
    cancel: &CancelToken,
) -> Result<(Vec<usize>, Option<xycut::Doubt>), ExtractError> {
    let cut = |boxes: &[RectPt], lines: &[usize]| {
        xycut::order(boxes, lines, rtl, options.limits.max_xycut_depth, cancel)
            .map_err(|xycut::Cancelled| ExtractError::Cancelled)
    };
    if options.reading_order == ReadingOrder::ContentStream {
        let mut order: Vec<usize> = (0..boxes.len()).collect();
        order.sort_by_key(|&i| first.get(i).copied().unwrap_or(usize::MAX));
        return Ok((order, None));
    }
    if region.iter().all(Option::is_none) {
        return cut(boxes, lines).map(|o| (o.order, o.doubt));
    }
    // One unit per region with items, and one per item outside every region.
    let mut units: Vec<Vec<usize>> = Vec::new();
    let mut unit_of_region = std::collections::BTreeMap::new();
    for (i, r) in region.iter().enumerate().take(boxes.len()) {
        match r {
            Some(r) => {
                let unit = *unit_of_region.entry(*r).or_insert_with(|| {
                    units.push(Vec::new());
                    units.len() - 1
                });
                if let Some(members) = units.get_mut(unit) {
                    members.push(i);
                }
            }
            None => units.push(vec![i]),
        }
    }
    let unit_boxes: Vec<RectPt> = units
        .iter()
        .map(|members| {
            members
                .iter()
                .filter_map(|&i| boxes.get(i).copied())
                .reduce(|a, b| a.union(&b))
                .unwrap_or_default()
        })
        .collect();
    let unit_lines: Vec<usize> = units
        .iter()
        .map(|members| members.iter().filter_map(|&i| lines.get(i)).sum())
        .collect();
    let mut order = Vec::with_capacity(boxes.len());
    // A layout model drew the regions, so the rules' doubts about columns don't apply.
    for u in cut(&unit_boxes, &unit_lines)?.order {
        let Some(members) = units.get_mut(u) else { continue };
        members.sort_by(|&a, &b| {
            let (a, b) = (
                boxes.get(a).copied().unwrap_or_default(),
                boxes.get(b).copied().unwrap_or_default(),
            );
            let across = if rtl {
                b.x0.get().total_cmp(&a.x0.get())
            } else {
                a.x0.get().total_cmp(&b.x0.get())
            };
            a.y0.get().total_cmp(&b.y0.get()).then(across)
        });
        order.extend(members.iter().copied());
    }
    Ok((order, None))
}

/// `items` taken in `order`.
fn take_in<T>(items: Vec<T>, order: &[usize]) -> Vec<T> {
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
    order
        .iter()
        .filter_map(|&i| slots.get_mut(i).and_then(Option::take))
        .collect()
}

/// The page's lines, built within each layout region (and among the pieces outside every
/// region), so no line crosses a region's edge; with each line's region. Without regions,
/// the rules' lines.
fn region_lines(pieces: &[Piece], regions: &[LayoutRegion], covered: &[RectPt]) -> (Vec<Line>, Vec<Option<usize>>) {
    if regions.is_empty() {
        let lines = lines(pieces, covered);
        let none = vec![None; lines.len()];
        return (lines, none);
    }
    let mut groups: std::collections::BTreeMap<Option<usize>, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, piece) in pieces.iter().enumerate() {
        groups
            .entry(region_near(regions, &piece.bbox, piece.size.get() / 2.0))
            .or_default()
            .push(i);
    }
    let (mut out, mut region) = (Vec::new(), Vec::new());
    for (r, members) in groups {
        let sub: Vec<Piece> = members.iter().filter_map(|&i| pieces.get(i).cloned()).collect();
        for mut line in lines(&sub, covered) {
            line.pieces = line.pieces.iter().filter_map(|&p| members.get(p).copied()).collect();
            out.push(line);
            region.push(r);
        }
    }
    (out, region)
}

/// The lines grouped into blocks, never across a layout region's edge: each region's lines
/// (and the lines outside every region) make their own blocks.
fn region_drafts(
    lines: &[Line],
    line_region: &[Option<usize>],
    vertical: impl Fn(&Line) -> bool,
) -> Vec<(Draft, Option<usize>)> {
    if line_region.iter().all(Option::is_none) {
        return blocks(lines, vertical).into_iter().map(|d| (d, None)).collect();
    }
    let mut groups: std::collections::BTreeMap<Option<usize>, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, r) in line_region.iter().enumerate().take(lines.len()) {
        groups.entry(*r).or_default().push(i);
    }
    let mut out = Vec::new();
    for (region, members) in groups {
        let sub: Vec<Line> = members.iter().filter_map(|&i| lines.get(i).cloned()).collect();
        for mut draft in blocks(&sub, &vertical) {
            draft.lines = draft.lines.iter().filter_map(|&l| members.get(l).copied()).collect();
            out.push((draft, region));
        }
    }
    out
}

/// The page's tables, from its rules (`edges`, collected while its text was read) and, on
/// request, its aligned text. Their units are the paragraph pieces, so a table's text is the
/// same text (ligatures spelled out, `ActualText` applied) and each piece lands in exactly one
/// of a cell and the paragraph flow.
fn find_tables(
    doc: &Document,
    index: usize,
    edges: Option<&Edges>,
    pieces: &[Piece],
    regions: &[LayoutRegion],
    options: &ExtractOptions,
) -> Result<Vec<Table>, ExtractError> {
    let Some(edges) = edges.filter(|_| options.tables != TableMode::Off && !pieces.is_empty()) else {
        return Ok(Vec::new());
    };
    let glyphs: Vec<Glyph> = pieces
        .iter()
        .map(|p| Glyph {
            text: p.text.clone(),
            bbox: p.bbox,
            size: p.size.get(),
            bold: is_bold(&p.font),
        })
        .collect();
    let max_cells = usize::try_from(options.limits.max_table_cells).unwrap_or(usize::MAX);
    let mut limited = false;
    let mut tables = tables::detect(options.tables, edges, &glyphs, max_cells, &mut limited);
    // A table region with no ruled table in it gets the unruled detector, inside the region.
    for region in regions.iter().filter(|r| r.label == RegionLabel::Table) {
        if tables.iter().any(|t| inside(&region.bbox, &t.bbox)) {
            continue;
        }
        let taken: std::collections::BTreeSet<usize> = tables.iter().flat_map(|t| t.glyphs.iter().copied()).collect();
        let members: Vec<usize> = (0..glyphs.len())
            .filter(|i| !taken.contains(i) && glyphs.get(*i).is_some_and(|g| inside(&region.bbox, &g.bbox)))
            .collect();
        tables.extend(tables::stream_in(&glyphs, &members));
    }
    if limited {
        doc.report(Warning::ExtractTableLimit {
            page: u64::try_from(index).unwrap_or(u64::MAX),
            limit: options.limits.max_table_cells,
        })?;
    }
    Ok(tables)
}

/// A table block's text: a tab between cells, a line break between rows.
fn table_text(table: &Table) -> String {
    let mut rows: Vec<Vec<&str>> = vec![Vec::new(); table.rows];
    for cell in &table.cells {
        if let Some(row) = rows.get_mut(cell.row) {
            row.push(&cell.text);
        }
    }
    rows.iter().map(|r| r.join("\t")).collect::<Vec<_>>().join("\n")
}

/// Extracts page `index`, whose text is `text`.
pub(crate) fn extract_page(
    doc: &Document,
    page: &Page<'_>,
    index: usize,
    text: &PageText,
    painting: &PageInput<'_>,
    options: &ExtractOptions,
    cancel: &CancelToken,
) -> Result<PageOut, ExtractError> {
    let frame = &text.frame;
    let text_layer = measure(text, painting.backdrops, painting.drawn, &options.text_layer);
    let info = ExtractedPage {
        index,
        number: index.saturating_add(1),
        label: page.label()?,
        width: frame.width(),
        height: frame.height(),
        coordinates: Coordinates::new(frame.rotate(), frame.user_unit(), frame.is_letter_fallback()),
        text_layer,
    };
    let Runs {
        mut pieces,
        mut spans,
        mut dropped,
        covered,
    } = runs(index, text, painting, options);
    let (ocr, ocr_spans) = ocr_pieces(painting.words);
    pieces.extend(ocr);
    spans.extend(ocr_spans);
    let page_number = u64::try_from(index).unwrap_or(u64::MAX);
    // A page whose text isn't its text says so, unless an OCR engine already read it: its
    // Markdown alone would just be empty.
    if matches!(text_layer.status, TextLayerStatus::NeedsOcr | TextLayerStatus::OcrLayer) && painting.words.is_empty() {
        doc.report(Warning::ExtractNeedsOcr {
            page: page_number,
            status: text_layer.status.as_str(),
        })?;
    }
    // Before the grouping and the cuts, whose work grows faster than the runs do.
    let limit = options.limits.max_blocks_per_page;
    let kept = usize::try_from(limit).unwrap_or(usize::MAX);
    if pieces.len() > kept {
        doc.report(Warning::ExtractLimitBlocks {
            page: page_number,
            limit,
        })?;
        pieces.truncate(kept);
        spans.truncate(kept);
    }
    let cx = PageContext {
        doc,
        info: &info,
        painting,
        covered: &covered,
        options,
        cancel,
    };
    let (blocks, tree_use) = match painting
        .tree
        .filter(|_| options.reading_order != ReadingOrder::ContentStream)
    {
        Some(tree) => {
            let check = options.reading_order != ReadingOrder::Tagged;
            match tagged::tree_blocks(tree, &info, pieces, spans, options.dehyphenate, check) {
                Ok(TreeBlocks {
                    mut blocks,
                    rest,
                    untagged,
                    tagged: true,
                }) => {
                    if untagged > 0 {
                        doc.report(Warning::ExtractUntaggedContent {
                            page: page_number,
                            chars: u64::try_from(untagged).unwrap_or(u64::MAX),
                        })?;
                    }
                    let (pieces, spans) = rest;
                    let (more, furniture) = layout(&cx, pieces, spans, blocks.len())?;
                    blocks.extend(more);
                    dropped.extend(furniture);
                    (blocks, TreeUse::Used)
                }
                Ok(TreeBlocks {
                    rest: (pieces, spans), ..
                }) => (
                    with_dropped(layout(&cx, pieces, spans, 0)?, &mut dropped),
                    TreeUse::None,
                ),
                Err((implausible, (pieces, spans))) => {
                    doc.report(Warning::ExtractStructTreeIgnored {
                        page: page_number,
                        reason: implausible.reason(),
                    })?;
                    (
                        with_dropped(layout(&cx, pieces, spans, 0)?, &mut dropped),
                        TreeUse::Ignored(implausible),
                    )
                }
            }
        }
        None => (
            with_dropped(layout(&cx, pieces, spans, 0)?, &mut dropped),
            TreeUse::None,
        ),
    };
    Ok(PageOut {
        info,
        blocks,
        dropped,
        tree: tree_use,
    })
}

/// What a page's layout reads besides its pieces.
struct PageContext<'a> {
    doc: &'a Document,
    info: &'a ExtractedPage,
    painting: &'a PageInput<'a>,
    /// The covered text's boxes, which never split a line ([`lines`]).
    covered: &'a [RectPt],
    options: &'a ExtractOptions,
    cancel: &'a CancelToken,
}

/// The layout's blocks of `pieces` (with their `spans`): lines, blocks, tables, XY-Cut++ order.
/// Their IDs count from `id_from`, which is 0 only when the layout reads the whole page (not the
/// text a structure tree left out), so only then are its column-order doubts reported.
fn layout(
    cx: &PageContext<'_>,
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    id_from: usize,
) -> Result<(Vec<Block>, Vec<Dropped>), ExtractError> {
    let (doc, info, painting, options, cancel) = (cx.doc, cx.info, cx.painting, cx.options, cx.cancel);
    let index = info.index;
    let page_number = u64::try_from(index).unwrap_or(u64::MAX);
    let tables = find_tables(doc, index, painting.edges, &pieces, painting.regions, options)?;
    if tables.iter().any(|t| t.ambiguous) {
        doc.report(Warning::ExtractTableAmbiguous { page: page_number })?;
    }
    let flow = Flow::split(pieces, spans, &tables);
    let regions = painting.regions;
    let (lines, line_region) = region_lines(&flow.pieces, regions, cx.covered);
    let vertical = lines.iter().filter(|l| is_vertical(l, &flow.pieces)).count();
    if vertical > 0 && vertical < lines.len() {
        doc.report(Warning::ExtractReadingOrderAmbiguous { page: page_number })?;
    }
    let drafts = region_drafts(&lines, &line_region, |l| is_vertical(l, &flow.pieces));
    // Vertical writing (Chinese, Japanese) sets its columns right to left, as do right-to-left
    // scripts.
    let rtl = 2 * vertical > lines.len() || mostly_rtl(&lines.iter().map(|l| l.text.as_str()).collect::<String>());
    let mut items: Vec<(Item, Option<usize>)> = drafts.into_iter().map(|(d, r)| (Item::Text(d), r)).collect();
    items.extend(
        tables
            .iter()
            .enumerate()
            .map(|(t, table)| (Item::Table(t), region_of(regions, &table.bbox))),
    );
    let (mut boxes, mut counts, mut first, mut region) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (item, r) in &items {
        let (bbox, count, start) = match item {
            Item::Text(d) => (d.bbox, d.lines.len(), flow.first_run(&d.lines, &lines)),
            Item::Table(t) => match tables.get(*t) {
                Some(table) => (
                    table.bbox,
                    table.rows.max(2),
                    flow.table_first.get(*t).copied().unwrap_or(usize::MAX),
                ),
                None => continue,
            },
        };
        boxes.push(bbox);
        counts.push(count);
        first.push(start);
        region.push(*r);
    }
    let (order, doubt) = order_of(&boxes, &counts, &first, &region, rtl, options, cancel)?;
    let doubted = report_doubts(cx, &items, &lines, doubt, id_from == 0, vertical > 0)?;
    let items = take_in(items.into_iter().zip(doubted).collect(), &order);
    let confidence = match options.reading_order {
        ReadingOrder::ContentStream => CONTENT_CONFIDENCE,
        ReadingOrder::Auto | ReadingOrder::Tagged | ReadingOrder::Layout => LAYOUT_CONFIDENCE,
    };
    let mut out = Vec::with_capacity(items.len());
    let mut furniture = Vec::new();
    for ((item, r), doubted) in items {
        let id = format!("p{index}-b{}", out.len() + id_from);
        match item {
            Item::Text(draft) => {
                let confidence = if doubted { UNCERTAIN_CONFIDENCE } else { confidence };
                let mut block = text_block(id, &draft, &lines, &flow, info, options, confidence);
                let Some(region) = r.and_then(|r| regions.get(r)) else {
                    out.push(block);
                    continue;
                };
                match kind_for(region.label, &block.text) {
                    Ok((kind, text)) => {
                        block.kind = kind;
                        block.from_model = true;
                        if let Some(text) = text {
                            block.text = text;
                        }
                        // The model placed it: its score, not the rules' fixed confidence, but
                        // never above what OCR'd words give.
                        if ocr_read(&block) {
                            block.confidence = block.confidence.min(unit(region.score));
                        } else {
                            block.confidence = unit(region.score);
                        }
                        out.push(block);
                    }
                    Err(reason) if options.drop.furniture => furniture.push(Dropped {
                        reason,
                        page: index,
                        bbox: block.bbox,
                        chars: visible_chars(&block.text),
                        spans: if options.drop.keep_spans {
                            block.spans
                        } else {
                            Vec::new()
                        },
                    }),
                    Err(_) => {
                        block.kind = BlockKind::Other;
                        out.push(block);
                    }
                }
            }
            Item::Table(t) => {
                if let (Some(table), Some(spans)) = (tables.get(t), flow.table_spans.get(t)) {
                    out.push(table_block(id, table, spans, info));
                }
            }
        }
    }
    Ok((out, furniture))
}

/// A score as a confidence: 0 to 1, and 0 for a model's NaN.
fn unit(score: f64) -> f64 {
    if score.is_finite() { score.clamp(0.0, 1.0) } else { 0.0 }
}

/// Whether an OCR engine read the block's text.
fn ocr_read(block: &Block) -> bool {
    block.spans.iter().any(|s| s.ocr.is_some())
}

/// A layout's blocks, its furniture moved into `dropped`.
fn with_dropped((blocks, furniture): (Vec<Block>, Vec<Dropped>), dropped: &mut Vec<Dropped>) -> Vec<Block> {
    dropped.extend(furniture);
    blocks
}

/// The page's pieces and spans with the tables' taken out, and the tables' own spans.
struct Flow {
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    /// Each kept piece's index among all the page's pieces (its drawing order).
    order: Vec<usize>,
    table_spans: Vec<Vec<Span>>,
    /// Each table's first piece in drawing order.
    table_first: Vec<usize>,
}

impl Flow {
    fn split(pieces: Vec<Piece>, spans: Vec<Span>, tables: &[Table]) -> Self {
        let mut flow = Self {
            pieces: Vec::with_capacity(pieces.len()),
            spans: Vec::with_capacity(spans.len()),
            order: Vec::with_capacity(pieces.len()),
            table_spans: vec![Vec::new(); tables.len()],
            table_first: vec![usize::MAX; tables.len()],
        };
        // Each piece's table, if a cell took it.
        let mut owner: Vec<Option<usize>> = vec![None; pieces.len()];
        for (t, table) in tables.iter().enumerate() {
            for &g in &table.glyphs {
                if let Some(slot) = owner.get_mut(g) {
                    *slot = Some(t);
                }
            }
        }
        for (i, (piece, span)) in pieces.into_iter().zip(spans).enumerate() {
            if let Some(t) = owner.get(i).copied().flatten() {
                if let Some(list) = flow.table_spans.get_mut(t) {
                    list.push(span);
                }
                if let Some(f) = flow.table_first.get_mut(t) {
                    *f = (*f).min(i);
                }
            } else {
                flow.pieces.push(piece);
                flow.spans.push(span);
                flow.order.push(i);
            }
        }
        flow
    }

    /// The first run in drawing order among a block's lines.
    fn first_run(&self, block_lines: &[usize], lines: &[Line]) -> usize {
        block_lines
            .iter()
            .filter_map(|&l| lines.get(l))
            .flat_map(|l| l.pieces.iter().filter_map(|&p| self.order.get(p).copied()))
            .min()
            .unwrap_or(usize::MAX)
    }
}

fn text_block(
    id: String,
    draft: &Draft,
    lines: &[Line],
    flow: &Flow,
    info: &ExtractedPage,
    options: &ExtractOptions,
    confidence: f64,
) -> Block {
    let block_lines: Vec<&Line> = draft.lines.iter().filter_map(|&l| lines.get(l)).collect();
    let text = join_lines(block_lines.iter().map(|l| l.text.as_str()), options.dehyphenate);
    let spans: Vec<Span> = block_lines
        .iter()
        .flat_map(|l| l.pieces.iter().filter_map(|&p| flow.spans.get(p).cloned()))
        .collect();
    // Text an OCR engine read is as sure as its words.
    let ocr: Vec<f64> = spans.iter().filter_map(|s| s.ocr).collect();
    #[expect(clippy::cast_precision_loss, reason = "a block's word count is far below 2^52")]
    let confidence = if ocr.is_empty() {
        confidence
    } else {
        ocr.iter().sum::<f64>() / ocr.len() as f64
    };
    Block {
        id,
        kind: BlockKind::Paragraph,
        text,
        page: info.index,
        bbox: draft.bbox,
        bbox_norm: normalised(&draft.bbox, info.width, info.height),
        struct_path: Vec::new(),
        struct_ref: None,
        spans,
        confidence,
        cells: Vec::new(),
        continues: None,
        continued_by: None,
        from_model: false,
    }
}

fn table_block(id: String, table: &Table, spans: &[Span], info: &ExtractedPage) -> Block {
    Block {
        id,
        kind: BlockKind::Table {
            rows: table.rows,
            cols: table.cols,
            detector: table.detector,
        },
        text: table_text(table),
        page: info.index,
        bbox: table.bbox,
        bbox_norm: normalised(&table.bbox, info.width, info.height),
        struct_path: Vec::new(),
        struct_ref: None,
        spans: spans.to_vec(),
        confidence: table.confidence,
        cells: table.cells.clone(),
        continues: None,
        continued_by: None,
        from_model: false,
    }
}
