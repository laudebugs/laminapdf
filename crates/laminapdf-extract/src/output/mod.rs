//! An [`Extraction`] as text: Markdown, plain text, retrieval-sized chunks and (with the
//! `serde` feature) `laminapdf.extract/1` JSON. Every rendering records where each block's text
//! sits in it ([`TextRange`]), so a chunk or a JSON block can point back into the Markdown.

mod chunks;
#[cfg(feature = "serde")]
mod json;
mod markdown;

pub use chunks::{Chunk, ChunkOptions, Region, SplitOn, tokens_approx};
#[cfg(feature = "schema")]
pub use json::json_schema;
#[cfg(feature = "serde")]
pub use json::{JsonOptions, chunks_jsonl, to_json};
pub use markdown::MarkdownOptions;

use crate::model::{Block, BlockKind, Extraction, header_row_count};

/// For each block, whether it is a table part that goes on from the block just before it.
fn joins(blocks: &[Block]) -> Vec<bool> {
    (0..blocks.len())
        .map(|i| i > 0 && blocks.get(i).zip(blocks.get(i - 1)).is_some_and(|(b, a)| b.continues_from(a)))
        .collect()
}

/// Where a piece of text sits in a rendering, in Unicode scalar values (`char`s), `start`
/// inclusive and `end` exclusive. Code points, not bytes or UTF-16 units, so Python slices a
/// rendering with them as they are; [`Rendered::slice`] does it in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct TextRange {
    /// The first character.
    pub start: usize,
    /// One past the last character.
    pub end: usize,
}

/// A rendering and each block's place in it, block by block as the extraction lists them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Rendered {
    /// The rendered text.
    pub text: String,
    /// Each block's range in `text`.
    pub ranges: Vec<TextRange>,
    /// The same ranges in bytes, for slicing.
    bytes: Vec<(usize, usize)>,
}

impl Rendered {
    /// The text in `range` (empty when it lies outside the rendering).
    #[must_use]
    pub fn slice(&self, range: TextRange) -> &str {
        let byte = |chars: usize| self.text.char_indices().nth(chars).map_or(self.text.len(), |(b, _)| b);
        self.text.get(byte(range.start)..byte(range.end)).unwrap_or_default()
    }

    /// Block `i`'s text in the rendering.
    #[must_use]
    pub fn block(&self, i: usize) -> &str {
        self.bytes
            .get(i)
            .and_then(|&(a, b)| self.text.get(a..b))
            .unwrap_or_default()
    }

    /// The bytes from the start of block `first` to the end of block `last`.
    fn span_bytes(&self, first: usize, last: usize) -> Option<(usize, usize)> {
        Some((self.bytes.get(first)?.0, self.bytes.get(last)?.1))
    }
}

/// Builds a rendering piece by piece, counting characters as it goes.
#[derive(Default)]
struct Writer {
    out: Rendered,
    chars: usize,
}

impl Writer {
    fn push(&mut self, s: &str) {
        self.out.text.push_str(s);
        self.chars += s.chars().count();
    }

    /// Writes one block's text and records its range.
    fn block(&mut self, s: &str) {
        let (start, byte) = (self.chars, self.out.text.len());
        self.push(s);
        self.out.ranges.push(TextRange { start, end: self.chars });
        self.out.bytes.push((byte, self.out.text.len()));
    }

    fn finish(mut self) -> Rendered {
        if !self.out.text.is_empty() {
            self.out.text.push('\n');
        }
        self.out
    }
}

impl Extraction {
    /// The blocks as Markdown: headings as `#` lines, list items as `-` or numbered items,
    /// tables as GitHub pipe tables (or HTML when a cell spans rows or columns), the rest as
    /// paragraphs, a blank line between blocks. Hidden text is never in it.
    #[must_use]
    pub fn to_markdown(&self, options: &MarkdownOptions) -> Rendered {
        markdown::render(self, *options)
    }

    /// The blocks' text in reading order, a blank line between blocks; a list item starts with
    /// its marker, and a table is its rows, a tab between cells. A table that goes on over a
    /// page break is one table: each later part's rows follow on the next line, without the
    /// header rows it repeats.
    #[must_use]
    pub fn to_text(&self) -> Rendered {
        let mut w = Writer::default();
        let joined = joins(&self.blocks);
        for (i, block) in self.blocks.iter().enumerate() {
            let joined = joined.get(i).copied().unwrap_or(false);
            if i > 0 {
                w.push(if joined { "\n" } else { "\n\n" });
            }
            match &block.kind {
                BlockKind::ListItem { marker, .. } => w.block(&format!("{marker} {}", block.text)),
                BlockKind::Table { .. } if joined => {
                    let skip = header_row_count(&block.cells);
                    w.block(&block.text.split('\n').skip(skip).collect::<Vec<_>>().join("\n"));
                }
                _ => w.block(&block.text),
            }
        }
        w.finish()
    }

    /// The Markdown cut into chunks for retrieval, each a contiguous piece of `markdown` (which
    /// must be [`Extraction::to_markdown`]'s output for this extraction).
    #[must_use]
    pub fn chunks(&self, markdown: &Rendered, options: &ChunkOptions) -> Vec<Chunk> {
        chunks::split(self, markdown, options)
    }
}

#[cfg(test)]
mod tests {
    use super::{TextRange, Writer};

    #[test]
    fn ranges_count_characters_not_bytes() {
        let mut w = Writer::default();
        w.block("Größe");
        w.push("\n\n");
        w.block("Été");
        let out = w.finish();
        assert_eq!(
            out.ranges,
            [TextRange { start: 0, end: 5 }, TextRange { start: 7, end: 10 }]
        );
        assert_eq!(out.slice(out.ranges[1]), "Été");
        assert_eq!(out.block(0), "Größe");
    }
}
