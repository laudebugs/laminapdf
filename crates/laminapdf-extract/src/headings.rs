//! Headings from font metrics ([ADR 0030] §3): the body size is the size that covers the most
//! characters; short blocks set larger, and bold blocks at the body size, are headings, ranked
//! by size into up to six levels.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use std::collections::BTreeMap;

use crate::lists::{Marker, split_marker};
use crate::model::{Block, BlockKind, Span};
use crate::tables::is_bold;
use crate::text_fix::visible_chars;

/// A heading is set at least this many times the body size, or bold at the body size.
const LARGER: f64 = 1.1;

/// Sizes within this fraction of each other are one heading level.
const SAME_LEVEL: f64 = 0.1;

/// A heading has at most this many lines.
const MAX_LINES: usize = 3;

/// A bold heading at the body size has at most this many lines.
const MAX_BOLD_LINES: usize = 2;

/// A heading has at most this many characters (spaces left out).
const MAX_CHARS: usize = 200;

/// The deepest level.
const MAX_LEVEL: usize = 6;

/// A size in half points, the step sizes are compared in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a font size in half points, far inside i64"
)]
fn half_points(size: f64) -> i64 {
    (size * 2.0).round() as i64
}

/// The size, in half points, that covers the most of `spans`' characters (the larger on a tie).
fn dominant<'a>(spans: impl IntoIterator<Item = &'a Span>) -> Option<(i64, usize)> {
    let mut chars: BTreeMap<i64, usize> = BTreeMap::new();
    for span in spans {
        *chars.entry(half_points(span.size.get())).or_default() += visible_chars(&span.text);
    }
    chars
        .into_iter()
        .filter(|&(_, n)| n > 0)
        .max_by_key(|&(size, n)| (n, size))
}

/// How many lines `spans` sit on: baselines further apart than half a size.
fn line_count(spans: &[Span]) -> usize {
    let mut baselines: Vec<(f64, f64)> = spans.iter().map(|s| (s.baseline.get(), s.size.get())).collect();
    baselines.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut count = 0;
    let mut last: Option<f64> = None;
    for (baseline, size) in baselines {
        if last.is_none_or(|l| baseline - l > 0.5 * size) {
            count += 1;
            last = Some(baseline);
        }
    }
    count
}

/// How a block could be a heading.
enum Candidate {
    /// Set larger than the body, at this size in half points.
    Larger(i64),
    /// Bold at the body size.
    Bold,
}

fn candidate(block: &Block, body: i64) -> Option<Candidate> {
    // A block from the structure tree has the kind the tree gives it.
    if block.kind != BlockKind::Paragraph
        || !block.struct_path.is_empty()
        || block.from_model
        || !block.text.chars().any(char::is_alphabetic)
    {
        return None;
    }
    // A bullet starts a list item, whatever its size or weight.
    let marker = split_marker(&block.text).map(|(_, kind, _)| kind);
    if matches!(marker, Some(Marker::Bullet | Marker::WeakBullet)) {
        return None;
    }
    let (size, _) = dominant(&block.spans)?;
    let lines = line_count(&block.spans);
    if lines > MAX_LINES || visible_chars(&block.text) > MAX_CHARS {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "sizes in half points, far below 2^52")]
    let (s, b) = (size as f64, body as f64);
    if s >= LARGER * b {
        return Some(Candidate::Larger(size));
    }
    let bold = block.spans.iter().all(|s| is_bold(&s.font));
    let sentence = block.text.trim_end().ends_with(['.', ',', ';']);
    // At the body size a numbered line is a list item, not a heading.
    (bold && size == body && lines <= MAX_BOLD_LINES && !sentence && marker.is_none()).then_some(Candidate::Bold)
}

/// Marks the headings among `blocks` (paragraphs only), with their levels.
pub(crate) fn assign(blocks: &mut [Block]) {
    let body = dominant(
        blocks
            .iter()
            .filter(|b| b.kind == BlockKind::Paragraph)
            .flat_map(|b| &b.spans),
    );
    let Some((body, _)) = body else {
        return;
    };
    let found: Vec<(usize, Candidate)> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| candidate(b, body).map(|c| (i, c)))
        .collect();
    // The larger sizes, largest first, grouped where they are within SAME_LEVEL of the
    // group's largest: each group is a level.
    let mut sizes: Vec<i64> = found
        .iter()
        .filter_map(|(_, c)| match c {
            Candidate::Larger(s) => Some(*s),
            Candidate::Bold => None,
        })
        .collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    sizes.dedup();
    let mut level_of: BTreeMap<i64, usize> = BTreeMap::new();
    let mut top: Option<i64> = None;
    let mut level = 0;
    for size in sizes {
        #[expect(clippy::cast_precision_loss, reason = "sizes in half points, far below 2^52")]
        let close = top.is_some_and(|t| size as f64 >= (1.0 - SAME_LEVEL) * t as f64);
        if !close {
            level += 1;
            top = Some(size);
        }
        level_of.insert(size, level.min(MAX_LEVEL));
    }
    let bold_level = (level + 1).min(MAX_LEVEL);
    for (i, c) in found {
        let level = match c {
            Candidate::Larger(s) => level_of.get(&s).copied().unwrap_or(MAX_LEVEL),
            Candidate::Bold => bold_level,
        };
        if let Some(block) = blocks.get_mut(i) {
            block.kind = BlockKind::Heading {
                level: u8::try_from(level).unwrap_or(6),
            };
        }
    }
}
