//! Chunks for retrieval: whole blocks grouped up to a token budget, each chunk a contiguous
//! piece of the Markdown with its provenance (blocks, headings above it, pages, boxes).

use std::collections::BTreeMap;

use laminapdf_model::RectPt;

use super::{Join, Rendered, TextRange, joins};
use crate::model::{BlockKind, Extraction};

/// Where chunks may start.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "kebab-case"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SplitOn {
    /// At every heading, and between blocks when the budget runs out.
    #[default]
    Headings,
    /// Only between blocks when the budget runs out.
    Blocks,
    /// At every page, and between blocks when the budget runs out.
    Pages,
}

/// How the Markdown is cut.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ChunkOptions {
    /// A chunk's budget in approximate tokens ([`tokens_approx`]). A block over budget is a
    /// chunk of its own: blocks are never cut.
    pub max_tokens: usize,
    /// How many approximate tokens of whole blocks a chunk repeats from the end of the one
    /// before it, when the budget (not a heading or a page) ended that one.
    pub overlap: usize,
    /// Where chunks may start.
    pub split_on: SplitOn,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap: 0,
            split_on: SplitOn::default(),
        }
    }
}

impl ChunkOptions {
    /// 512 tokens, no overlap, split at headings.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The budget.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// The overlap.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_overlap(mut self, overlap: usize) -> Self {
        self.overlap = overlap;
        self
    }

    /// Where chunks may start.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_split_on(mut self, split_on: SplitOn) -> Self {
        self.split_on = split_on;
        self
    }
}

/// A chunk's area on one page.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Region {
    /// The page index.
    pub page: usize,
    /// The union of the chunk's blocks' boxes on that page, in page space.
    #[cfg_attr(feature = "serde", serde(serialize_with = "crate::model::ser::rect"))]
    #[cfg_attr(feature = "schema", schemars(with = "[f64; 4]"))]
    pub bbox: RectPt,
}

/// One chunk.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct Chunk {
    /// `c<n>`, from 0, in order.
    pub id: String,
    /// The Markdown from the first block's start to the last block's end.
    pub text: String,
    /// The blocks it holds, in order.
    pub block_ids: Vec<String>,
    /// The headings above its first block, outermost first.
    pub heading_path: Vec<String>,
    /// The pages its blocks are on, ascending.
    pub pages: Vec<usize>,
    /// Its area on each of those pages.
    pub regions: Vec<Region>,
    /// Where `text` sits in the Markdown.
    pub text_range: TextRange,
    /// [`tokens_approx`] of `text`.
    pub tokens_approx: usize,
}

/// About how many tokens `text` is for an LLM's tokenizer: one per four characters, rounded up.
/// English prose runs close to that with today's tokenizers; code, numbers and scripts other
/// than Latin run higher, so leave room in a budget.
///
/// Added in 0.2.0.
#[must_use]
pub fn tokens_approx(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

pub(super) fn split(extraction: &Extraction, markdown: &Rendered, options: &ChunkOptions) -> Vec<Chunk> {
    let blocks = &extraction.blocks;
    let tokens: Vec<usize> = (0..blocks.len()).map(|i| tokens_approx(markdown.block(i))).collect();
    // A table part that goes on from the block before never starts a chunk, so a table split
    // over a page break stays in one, even past the budget.
    let joins = joins(blocks);
    let continues = |i: usize| joins.get(i).is_some_and(|&j| j != Join::No);
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut start = 0;
    let mut used = 0;
    for (i, block) in blocks.iter().enumerate() {
        let cost = tokens.get(i).copied().unwrap_or(0);
        let boundary = i > start
            && !continues(i)
            && match options.split_on {
                SplitOn::Headings => matches!(block.kind, BlockKind::Heading { .. }),
                SplitOn::Pages => blocks.get(i - 1).is_some_and(|prev| prev.page != block.page),
                SplitOn::Blocks => false,
            };
        let full = i > start && !continues(i) && used + cost > options.max_tokens;
        if boundary || full {
            groups.push((start, i - 1));
            start = i;
            used = 0;
            if full && !boundary {
                // Repeat whole blocks from the end of the chunk just closed, within the overlap.
                let mut back = 0;
                while start > 0 && start - 1 > groups.last().map_or(0, |g| g.0) {
                    let t = tokens.get(start - 1).copied().unwrap_or(0);
                    if back + t > options.overlap {
                        break;
                    }
                    back += t;
                    start -= 1;
                }
                // Nor does the overlap start in the middle of a table.
                while start < i && continues(start) {
                    back = back.saturating_sub(tokens.get(start).copied().unwrap_or(0));
                    start += 1;
                }
                used = back;
            }
        }
        used += cost;
    }
    if start < blocks.len() {
        groups.push((start, blocks.len() - 1));
    }
    groups
        .into_iter()
        .enumerate()
        .filter_map(|(n, (first, last))| chunk(extraction, markdown, n, first, last))
        .collect()
}

fn chunk(extraction: &Extraction, markdown: &Rendered, n: usize, first: usize, last: usize) -> Option<Chunk> {
    let blocks = extraction.blocks.get(first..=last)?;
    let (a, b) = markdown.span_bytes(first, last)?;
    let text = markdown.text.get(a..b)?.to_owned();
    let start = markdown.ranges.get(first)?.start;
    let end = markdown.ranges.get(last)?.end;
    let mut regions: BTreeMap<usize, RectPt> = BTreeMap::new();
    for block in blocks {
        regions
            .entry(block.page)
            .and_modify(|r| *r = r.union(&block.bbox))
            .or_insert(block.bbox);
    }
    Some(Chunk {
        id: format!("c{n}"),
        tokens_approx: tokens_approx(&text),
        text,
        block_ids: blocks.iter().map(|b| b.id.clone()).collect(),
        heading_path: heading_path(extraction, first),
        pages: regions.keys().copied().collect(),
        regions: regions.into_iter().map(|(page, bbox)| Region { page, bbox }).collect(),
        text_range: TextRange { start, end },
    })
}

/// The headings in force at block `at` (itself included when it is one), outermost first: each
/// heading replaces those at its level and below.
fn heading_path(extraction: &Extraction, at: usize) -> Vec<String> {
    let mut path: Vec<(u8, &str)> = Vec::new();
    for block in extraction.blocks.iter().take(at.saturating_add(1)) {
        if let BlockKind::Heading { level } = block.kind {
            path.retain(|&(l, _)| l < level);
            path.push((level, &block.text));
        }
    }
    path.into_iter().map(|(_, t)| t.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::tokens_approx;

    #[test]
    fn tokens_are_a_quarter_of_the_characters_rounded_up() {
        assert_eq!(tokens_approx(""), 0);
        assert_eq!(tokens_approx("abcd"), 1);
        assert_eq!(tokens_approx("abcde"), 2);
        assert_eq!(tokens_approx("Größe"), 2);
    }
}
