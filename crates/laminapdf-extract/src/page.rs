//! One page: its runs flagged or kept, kept runs grouped into lines and blocks, the blocks put
//! in reading order.

use laminapdf_cos::Warning;
use laminapdf_model::{Document, Page, Pt};
use laminapdf_syntax::CancelToken;
use laminapdf_text::extract::PageText;

use crate::blocks::{Draft, blocks};
use crate::error::ExtractError;
use crate::lines::{Line, Piece, lines, mostly_rtl};
use crate::model::{Block, BlockKind, Coordinates, Dropped, ExtractedPage, Span};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::text_fix::{decompose_ligatures, join_lines};
use crate::visibility::{flags, normalised};
use crate::xycut;

/// What one page gave.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageOut {
    pub(crate) info: ExtractedPage,
    pub(crate) blocks: Vec<Block>,
    pub(crate) dropped: Vec<Dropped>,
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

fn runs(index: usize, text: &PageText, options: &ExtractOptions) -> Runs {
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
        let words = if run.actual_text {
            run.text.clone()
        } else {
            decompose_ligatures(&run.text)
        };
        let span_flags = flags(run.invisible, &run.bounds, size, width, height);
        let span = Span {
            text: words,
            bbox: run.bounds,
            baseline,
            font: run.font_name.to_string(),
            size,
            flags: span_flags,
        };
        if let Some(&reason) = span_flags.reasons().first() {
            out.dropped.push(Dropped {
                reason,
                page: index,
                bbox: run.bounds,
                chars: span.text.chars().filter(|c| !c.is_whitespace()).count(),
                spans: if options.drop.keep_spans {
                    vec![span]
                } else {
                    Vec::new()
                },
            });
            continue;
        }
        if span.text.trim().is_empty() {
            continue;
        }
        out.pieces.push(Piece {
            text: span.text.clone(),
            bbox: span.bbox,
            baseline,
            size,
            font: span.font.clone(),
            vertical: run.vertical,
        });
        out.spans.push(span);
    }
    out
}

fn is_vertical(line: &Line, pieces: &[Piece]) -> bool {
    line.pieces
        .first()
        .and_then(|&i| pieces.get(i))
        .is_some_and(|p| p.vertical)
}

/// The drafts in reading order.
fn ordered(
    drafts: Vec<Draft>,
    lines: &[Line],
    rtl: bool,
    options: &ExtractOptions,
    cancel: &CancelToken,
) -> Result<Vec<Draft>, ExtractError> {
    let order: Vec<usize> = match options.reading_order {
        ReadingOrder::ContentStream => {
            // The first run each block holds, in the order the content stream drew the runs.
            let first = |d: &Draft| {
                d.lines
                    .iter()
                    .filter_map(|&l| lines.get(l))
                    .flat_map(|l| l.pieces.iter().copied())
                    .min()
                    .unwrap_or(usize::MAX)
            };
            let mut order: Vec<usize> = (0..drafts.len()).collect();
            order.sort_by_key(|&i| drafts.get(i).map_or(usize::MAX, first));
            order
        }
        ReadingOrder::Auto | ReadingOrder::Layout => {
            let boxes: Vec<_> = drafts.iter().map(|d| d.bbox).collect();
            let counts: Vec<_> = drafts.iter().map(|d| d.lines.len()).collect();
            xycut::order(&boxes, &counts, rtl, options.limits.max_xycut_depth, cancel)
                .map_err(|xycut::Cancelled| ExtractError::Cancelled)?
        }
    };
    let mut slots: Vec<Option<Draft>> = drafts.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .filter_map(|i| slots.get_mut(i).and_then(Option::take))
        .collect())
}

/// Extracts page `index`, whose text is `text`.
pub(crate) fn extract_page(
    doc: &Document,
    page: &Page<'_>,
    index: usize,
    text: &PageText,
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
    let Runs { pieces, spans, dropped } = runs(index, text, options);
    let lines = lines(&pieces);
    let vertical = lines.iter().filter(|l| is_vertical(l, &pieces)).count();
    let page_number = u64::try_from(index).unwrap_or(u64::MAX);
    if vertical > 0 && vertical < lines.len() {
        doc.report(Warning::ExtractReadingOrderAmbiguous { page: page_number })?;
    }
    let drafts = blocks(&lines, |l| is_vertical(l, &pieces));
    let rtl = mostly_rtl(&lines.iter().map(|l| l.text.as_str()).collect::<String>());
    let mut drafts = ordered(drafts, &lines, rtl, options, cancel)?;
    let limit = options.limits.max_blocks_per_page;
    if drafts.len() > usize::try_from(limit).unwrap_or(usize::MAX) {
        doc.report(Warning::ExtractLimitBlocks {
            page: page_number,
            limit,
        })?;
        drafts.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    }
    let confidence = match options.reading_order {
        ReadingOrder::ContentStream => CONTENT_CONFIDENCE,
        ReadingOrder::Auto | ReadingOrder::Layout => LAYOUT_CONFIDENCE,
    };
    let blocks = drafts
        .into_iter()
        .enumerate()
        .map(|(n, draft)| {
            let block_lines: Vec<&Line> = draft.lines.iter().filter_map(|&l| lines.get(l)).collect();
            let text = join_lines(block_lines.iter().map(|l| l.text.as_str()), options.dehyphenate);
            let spans = block_lines
                .iter()
                .flat_map(|l| l.pieces.iter().filter_map(|&p| spans.get(p).cloned()))
                .collect();
            Block {
                id: format!("p{index}-b{n}"),
                kind: BlockKind::Paragraph,
                text,
                page: index,
                bbox: draft.bbox,
                bbox_norm: normalised(&draft.bbox, info.width, info.height),
                struct_path: Vec::new(),
                struct_ref: None,
                spans,
                confidence,
            }
        })
        .collect();
    Ok(PageOut { info, blocks, dropped })
}
