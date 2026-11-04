# laminapdf-extract

`laminapdf-extract` turns a PDF's text into blocks a retrieval pipeline or an LLM can use: paragraphs in reading order, each with a box in top-left points it can cite, and no text a reader can't see. It is part of Lamina PDF's pure core (bytes in, bytes out, no models), and it implements the `laminapdf.extract/1` output that [ADR 0030](../../docs/decisions/0030-extraction-output-and-model-plugins.md) defines.

```rust
use laminapdf_extract::{ExtractOptions, extract};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::CancelToken;

let doc = Document::open(bytes, OpenOptions::default())?;
let out = extract(&doc, &ExtractOptions::new(), &CancelToken::new())?;
for block in &out.blocks {
    println!("page {} {:?}: {}", block.page, block.bbox, block.text);
}
```

For long documents, `extract_job` returns an `ExtractJob` that you step one page per work unit and can cancel between steps ([ADR 0028](../../docs/decisions/0028-facade-rules-jobs-and-cancellation.md)).

## How it reads a page

1. **Runs.** `laminapdf-text` gives the page's runs of glyphs with their boxes. Extraction leaves out any run drawn invisibly (text rendering mode 3 or 7), wholly off the page or below 1 pt. Each such run gets a `dropped` entry with its reason, its box and a character count, never its text. Ask for the hidden runs themselves with `DropOptions::keep_spans`.
2. **Lines.** Runs whose baselines lie within half a font size join one line, in visual order, right to left when most of the line's letters are. A gap wider than one font size between two runs on one baseline is a column gutter, so it splits them into two lines.
3. **Blocks.** A line joins the block above when it follows at most 1.6 font sizes lower, at a size within 20%, overlapping the block's last line by at least 30% of the narrower of the two.
4. **Order.** XY-Cut++ orders the blocks. Blocks that span two columns (titles, abstracts, full-width figures) split the page into bands. Each band is cut at its widest gaps, columns first, so a two-column page reads column by column; a grid of one-line blocks (a table) reads row by row; right-to-left pages take their columns from the right. `ReadingOrder::ContentStream` keeps the order the page draws instead.
5. **Text.** A block's lines are joined with spaces. A word hyphenated before a lowercase line is joined (`extrac-` / `tion`), while a hyphen before anything else stays (`well-` / `Known`). Latin ligatures are spelled out unless `/ActualText` gave the text.

On the reading-order corpus in `fixtures/reading-order/`, every file reads in order, including the copies drawn in reverse: one, two and three columns, an academic page, a sidebar, a table, a rotated page, and Arabic, Hebrew and Chinese text.

## What it can't do yet

- **Structure.** It ignores structure trees, and every block is a paragraph. Reading order from the structure tree, headings, list items, running headers and footers, and clipped or white-on-white text come in a later release; until then `structSource` is never `tagged`.
- **Hard layouts.** Rules fail on magazines, floating figures and text that wraps around shapes. Paragraphs without extra space between them merge into one block. Every block carries a `confidence` below 1 to say so; optional model plug-ins, planned for a later release, are the answer for those pages.
- **Vertical writing.** Each vertical run is a block of its own, and a page that mixes vertical and horizontal writing raises `extract-reading-order-ambiguous`.
- **Output formats.** It returns Rust values (serde behind the `serde` feature). Markdown, JSON and chunk serialisers, tables, and access through the `laminapdf` crate, WebAssembly and TypeScript come in a later release.

## Limits

`ExtractLimits` bounds the work per page: `max_blocks_per_page` (10,000; a page with more runs of text keeps its first runs, with `extract-limit-blocks`, before any grouping) and `max_xycut_depth` (64; a deeper group reads top to bottom).

## Tests

- `tests/reading_order.rs`: every file of the reading-order corpus reads in order by layout, and content order follows the drawing.
- `tests/extract.rs`: hidden text, hyphenation, a quarter-turned page, `/UserUnit`, determinism, jobs and cancellation, page selection and the blocks limit, on files built with `laminapdf-build`.
- `tests/regressions.rs`: the `extract` fuzz target's entry point over the corpus and every saved crash input.
