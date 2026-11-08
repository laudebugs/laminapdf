//! One page: its runs flagged or kept, kept runs grouped into lines and blocks, the blocks put
//! in reading order.

use laminapdf_cos::Warning;
use laminapdf_model::RectPt;
use laminapdf_model::{Document, Page, Pt};
use laminapdf_syntax::CancelToken;
use laminapdf_text::{PageText, PageTextRun, Quad, is_right_to_left};

use crate::blocks::{Draft, blocks};
use crate::error::ExtractError;
use crate::lines::{GUTTER, Line, Piece, SPACE, lines, mostly_rtl};
use crate::model::{Block, BlockKind, Coordinates, DropReason, Dropped, ExtractedPage, Span};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::tables::{self, Edges, Glyph, Table, TableMode, is_bold};
use crate::tagged::{self, TaggedIndex, TreeBlocks};
use crate::text_fix::{decompose_ligatures, join_lines, visible_chars};
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
    Ignored,
    /// No tree, or nothing of the tree on this page.
    None,
}

/// How sure the layout rules are of a block (ADR 0030 §1: below the structure tree's 1).
const LAYOUT_CONFIDENCE: f64 = 0.75;

/// Content order is the drawing order, which is often not the reading order.
const CONTENT_CONFIDENCE: f64 = 0.5;

/// The page's runs as spans: kept ones (with their pieces) and dropped ones.
struct Runs {
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    dropped: Vec<Dropped>,
}

/// What extraction reads of a page besides its text: what it painted, seen in the same walk of
/// its content, and the document's structure tree.
pub(crate) struct PageInput<'a> {
    /// The table rules, when tables are looked for.
    pub(crate) edges: Option<&'a Edges>,
    /// What text can stand on, for the low-contrast flag.
    pub(crate) backdrops: &'a Backdrops,
    /// The structure tree, when the reading order may come from it.
    pub(crate) tree: Option<&'a TaggedIndex>,
}

fn runs(index: usize, text: &PageText, backdrops: &Backdrops, options: &ExtractOptions) -> Runs {
    let frame = &text.frame;
    let (width, height, unit) = (frame.width(), frame.height(), frame.user_unit());
    let mut out = Runs {
        pieces: Vec::new(),
        spans: Vec::new(),
        dropped: Vec::new(),
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
        };
        // Artifacts are furniture by the file's own word (ISO 32000-2 §14.8.2.2).
        let artifact = (run.artifact && options.drop.furniture).then_some(DropReason::Artifact);
        if let Some(reason) = span_flags.reasons().first().copied().or(artifact) {
            out.dropped.push(Dropped {
                reason,
                page: index,
                bbox: run.bounds,
                chars: visible_chars(&span.text),
                spans: if options.drop.keep_spans {
                    vec![span]
                } else {
                    Vec::new()
                },
            });
            continue;
        }
        for (words, bbox) in segments(text, run, size) {
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

/// The smallest rectangle holding a glyph's quad.
fn quad_box(quad: &Quad) -> RectPt {
    let (x0, x1) = quad.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
        (a.min(p.x.get()), b.max(p.x.get()))
    });
    let (y0, y1) = quad.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
        (a.min(p.y.get()), b.max(p.y.get()))
    });
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// A visible run's text and box, split where its pen jumps forward past a column gutter:
/// `laminapdf-text` keeps such a jump inside one run (as a space), so two columns drawn row
/// by row in one font would otherwise share every line. Runs whose text is not their glyphs'
/// in drawing order (`ActualText`, right to left, vertical) stay whole.
fn segments(text: &PageText, run: &PageTextRun, size: Pt) -> Vec<(String, RectPt)> {
    let whole = || vec![(run.text.clone(), run.bounds)];
    if run.actual_text || run.vertical || run.text.chars().any(is_right_to_left) {
        return whole();
    }
    let Some(glyphs) = text.glyphs.get(run.glyphs.clone()) else {
        return whole();
    };
    let (gutter, space) = (GUTTER * size.get(), SPACE * size.get());
    let mut parts: Vec<(String, RectPt)> = Vec::new();
    for glyph in glyphs {
        let r = quad_box(&glyph.quad);
        match parts.last_mut() {
            Some((words, bbox)) if r.x0.get() - bbox.x1.get() <= gutter => {
                let gap = r.x0.get() - bbox.x1.get();
                if gap > space && !words.ends_with(char::is_whitespace) && !glyph.text.starts_with(char::is_whitespace)
                {
                    words.push(' ');
                }
                words.push_str(&glyph.text);
                *bbox = bbox.union(&r);
            }
            _ => parts.push((glyph.text.clone(), r)),
        }
    }
    if parts.len() <= 1 { whole() } else { parts }
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

/// The items in reading order. `first` is each item's first run in drawing order (for
/// [`ReadingOrder::ContentStream`]), `lines` its line count (a table counts its rows).
fn ordered(
    items: Vec<Item>,
    boxes: &[RectPt],
    lines: &[usize],
    first: &[usize],
    rtl: bool,
    options: &ExtractOptions,
    cancel: &CancelToken,
) -> Result<Vec<Item>, ExtractError> {
    let order: Vec<usize> = match options.reading_order {
        ReadingOrder::ContentStream => {
            let mut order: Vec<usize> = (0..items.len()).collect();
            order.sort_by_key(|&i| first.get(i).copied().unwrap_or(usize::MAX));
            order
        }
        ReadingOrder::Auto | ReadingOrder::Tagged | ReadingOrder::Layout => {
            xycut::order(boxes, lines, rtl, options.limits.max_xycut_depth, cancel)
                .map_err(|xycut::Cancelled| ExtractError::Cancelled)?
        }
    };
    let mut slots: Vec<Option<Item>> = items.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .filter_map(|i| slots.get_mut(i).and_then(Option::take))
        .collect())
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
    let tables = tables::detect(options.tables, edges, &glyphs, max_cells, &mut limited);
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
    let info = ExtractedPage {
        index,
        number: index.saturating_add(1),
        label: page.label()?,
        width: frame.width(),
        height: frame.height(),
        coordinates: Coordinates::new(frame.rotate(), frame.user_unit(), frame.is_letter_fallback()),
    };
    let Runs {
        mut pieces,
        mut spans,
        dropped,
    } = runs(index, text, painting.backdrops, options);
    let page_number = u64::try_from(index).unwrap_or(u64::MAX);
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
                    blocks.extend(layout(&cx, pieces, spans, blocks.len())?);
                    (blocks, TreeUse::Used)
                }
                Ok(TreeBlocks {
                    rest: (pieces, spans), ..
                }) => (layout(&cx, pieces, spans, 0)?, TreeUse::None),
                Err((implausible, (pieces, spans))) => {
                    doc.report(Warning::ExtractStructTreeIgnored {
                        page: page_number,
                        reason: implausible.reason(),
                    })?;
                    (layout(&cx, pieces, spans, 0)?, TreeUse::Ignored)
                }
            }
        }
        None => (layout(&cx, pieces, spans, 0)?, TreeUse::None),
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
    options: &'a ExtractOptions,
    cancel: &'a CancelToken,
}

/// The layout's blocks of `pieces` (with their `spans`): lines, blocks, tables, XY-Cut++ order.
/// Their IDs count from `id_from`.
fn layout(
    cx: &PageContext<'_>,
    pieces: Vec<Piece>,
    spans: Vec<Span>,
    id_from: usize,
) -> Result<Vec<Block>, ExtractError> {
    let (doc, info, painting, options, cancel) = (cx.doc, cx.info, cx.painting, cx.options, cx.cancel);
    let index = info.index;
    let page_number = u64::try_from(index).unwrap_or(u64::MAX);
    let tables = find_tables(doc, index, painting.edges, &pieces, options)?;
    if tables.iter().any(|t| t.ambiguous) {
        doc.report(Warning::ExtractTableAmbiguous { page: page_number })?;
    }
    let flow = Flow::split(pieces, spans, &tables);
    let lines = lines(&flow.pieces);
    let vertical = lines.iter().filter(|l| is_vertical(l, &flow.pieces)).count();
    if vertical > 0 && vertical < lines.len() {
        doc.report(Warning::ExtractReadingOrderAmbiguous { page: page_number })?;
    }
    let drafts = blocks(&lines, |l| is_vertical(l, &flow.pieces));
    // Vertical writing (Chinese, Japanese) sets its columns right to left, as do right-to-left
    // scripts.
    let rtl = 2 * vertical > lines.len() || mostly_rtl(&lines.iter().map(|l| l.text.as_str()).collect::<String>());
    let mut items: Vec<Item> = drafts.into_iter().map(Item::Text).collect();
    items.extend((0..tables.len()).map(Item::Table));
    let (mut boxes, mut counts, mut first) = (Vec::new(), Vec::new(), Vec::new());
    for item in &items {
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
    }
    let items = ordered(items, &boxes, &counts, &first, rtl, options, cancel)?;
    let confidence = match options.reading_order {
        ReadingOrder::ContentStream => CONTENT_CONFIDENCE,
        ReadingOrder::Auto | ReadingOrder::Tagged | ReadingOrder::Layout => LAYOUT_CONFIDENCE,
    };
    let blocks = items
        .into_iter()
        .enumerate()
        .filter_map(|(n, item)| {
            let id = format!("p{index}-b{}", n + id_from);
            match item {
                Item::Text(draft) => Some(text_block(id, &draft, &lines, &flow, info, options, confidence)),
                Item::Table(t) => Some(table_block(id, tables.get(t)?, flow.table_spans.get(t)?, info)),
            }
        })
        .collect();
    Ok(blocks)
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
    let spans = block_lines
        .iter()
        .flat_map(|l| l.pieces.iter().filter_map(|&p| flow.spans.get(p).cloned()))
        .collect();
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
    }
}
