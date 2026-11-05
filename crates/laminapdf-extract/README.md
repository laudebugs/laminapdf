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

For long documents, `extract_job` returns an `ExtractJob` that you step one page per work unit and can cancel between steps ([ADR 0028](../../docs/decisions/0028-facade-rules-jobs-and-cancellation.md)). Most programs use it through the `laminapdf` crate instead: `Document::extract`, `extract_job` and `extract_json`.

## Output

An `Extraction` renders three ways, and each rendering records where every block's text sits in it (a `TextRange`, in Unicode code points, so Python slices with it as is):

- **Markdown** (`to_markdown`): headings as `#` lines, list items, paragraphs, and tables as GitHub pipe tables, or HTML when a cell spans rows or columns. `MarkdownOptions::with_page_markers` writes `<!-- page N -->` before each page. Text is escaped only where it would change the structure.
- **Plain text** (`to_text`): blocks separated by a blank line; a table is its rows, a tab between cells.
- **Chunks** (`chunks`): whole blocks grouped up to a token budget (`ChunkOptions`: `max_tokens`, `overlap`, `split_on` headings, blocks or pages). A chunk is a contiguous piece of the Markdown, with its block IDs, the headings above it, its pages and a box per page. Tokens are approximated as a quarter of the characters, which runs low for code, numbers and scripts other than Latin.

With the `serde` feature, `to_json` writes the `laminapdf.extract/1` document, and `chunks_jsonl` the chunks as JSON Lines. The JSON Schema is `schemas/extract.schema.json`, generated from the Rust types with the `schema` feature.

## Tables

Extraction finds tables drawn with lines from the page's paths: stroked lines and rectangles, and filled rectangles thin enough to be a rule. Rules within 3 pt share a coordinate; collinear rules that nearly meet are joined; rules that cross form a group, and a group's distinct coordinates are the grid. A missing inner rule merges the cells on either side. Each cell takes the glyphs whose centres it holds, and a first row in bold or shaded is a header.

The rules refuse what isn't a grid of text: a page border, a form's separate field boxes, a one-row strip and an empty grid give no table. When the merged cells aren't rectangles, the table keeps one cell per square of the grid and raises `extract-table-ambiguous`.

`TableMode::LatticeAndStream` also finds tables drawn without lines: three or more lines that split at wide gaps into the same two or more columns. Stream detection is off by default because it sometimes takes two columns of text for a table; such tables carry a lower `confidence`.

## How it reads a page

1. **Runs.** `laminapdf-text` gives the page's runs of glyphs with their boxes. Extraction leaves out any run drawn invisibly (text rendering mode 3 or 7), wholly off the page or below 1 pt. Each such run gets a `dropped` entry with its reason, its box and a character count, never its text. Ask for the hidden runs themselves with `DropOptions::keep_spans`.
2. **Lines.** Runs whose baselines lie within half a font size join one line, in visual order, right to left when most of the line's letters are. A gap wider than one font size between two runs on one baseline is a column gutter, so it splits them into two lines.
3. **Blocks.** A line joins the block above when it follows at most 1.6 font sizes lower, at a size within 20%, overlapping the block's last line by at least 30% of the narrower of the two.
4. **Order.** XY-Cut++ orders the blocks. Blocks that span two columns (titles, abstracts, full-width figures) split the page into bands. Each band is cut at its widest gaps, columns first, so a two-column page reads column by column; a grid of one-line blocks (a table) reads row by row; right-to-left pages take their columns from the right. `ReadingOrder::ContentStream` keeps the order the page draws instead.
5. **Text.** A block's lines are joined with spaces. A word hyphenated before a lowercase line is joined (`extrac-` / `tion`), while a hyphen before anything else stays (`well-` / `Known`). Latin ligatures are spelled out unless `/ActualText` gave the text.

On the reading-order corpus in `fixtures/reading-order/`, every file reads in order, including the copies drawn in reverse: one, two and three columns, an academic page, a sidebar, a table, a rotated page, and Arabic, Hebrew and Chinese text.

## Benchmark

On [olmOCR-bench](https://huggingface.co/datasets/allenai/olmOCR-bench), the rules pass half of the reading-order tests and a quarter of the table tests. The benchmark's own tests scored Lamina PDF 0.2.0-alpha.2 on 2026-11-05, on the three subsets of born-digital pages. The data is revision `54a96a6fb6a2` of the dataset, by the Allen Institute for AI, under ODC-BY-1.0:

| Subset | What it measures | Pages | Tests | Ruled tables (the default) | Also tables without lines |
| --- | --- | ---: | ---: | ---: | ---: |
| `multi_column` | one passage before another, on multi-column pages | 231 | 884 | 52.8 % | 15.0 % |
| `table_tests` | a table cell's neighbours and headings | 188 | 1,022 | 24.1 % | 47.1 % |
| `headers_footers` | running headers, footers and page numbers left out | 266 | 760 | 37.6 % | 37.2 % |
| **Mean** | | | | **38.2 %** | **33.1 %** |

Three things explain most of the failures:

- **Pages with no text.** 89 of the 685 pages give no text. Most are scans, which need OCR. The other 13 store their content with a filter this crate can't decode yet (ASCII85 or LZW), and one such page stops the whole extraction.
- **No headers, footers or structure yet.** Running headers and page numbers stay in the text, which fails the third subset, until the extractor reads them and the structure tree.
- **Tables without lines.** Most of the benchmark's tables have no ruling lines, so the default finds few of them. Looking for text in columns doubles the table score but reads multi-column pages as tables, which wrecks their reading order. That trade is why it stays off by default.

The benchmark's published table scores OCR and vision models on all seven subsets, scans and equations included, so its numbers don't compare with these. `cargo xtask extract-bench` reruns this table; [`tools/bench/`](../../tools/bench/) says how, and why the other datasets weren't used.

## What it can't do yet

- **Structure.** It ignores structure trees, and every block is a paragraph. Reading order from the structure tree, headings, list items, running headers and footers, and clipped or white-on-white text come in a later release; until then `structSource` is never `tagged`.
- **Hard layouts.** Rules fail on magazines, floating figures and text that wraps around shapes. Paragraphs without extra space between them merge into one block. Every block carries a `confidence` below 1 to say so; optional model plug-ins, planned for a later release, are the answer for those pages.
- **Vertical writing.** Each vertical run is a block of its own, and a page that mixes vertical and horizontal writing raises `extract-reading-order-ambiguous`.
- **Tables without lines or with structure.** Unruled tables need `TableMode::LatticeAndStream`, and the structure tree's `Table` elements aren't read yet.

## Limits

`ExtractLimits` bounds the work per page: `max_blocks_per_page` (10,000; a page with more runs of text keeps its first runs, with `extract-limit-blocks`, before any grouping), `max_xycut_depth` (64; a deeper group reads top to bottom) and `max_table_cells` (10,000; a grid with more cells, or a page whose rules would take more than 16 times as many comparisons, gives no table, with `extract-table-limit`).

## Tests

- `tests/reading_order.rs`: every file of the reading-order corpus reads in order by layout, and content order follows the drawing.
- `tests/extract.rs`: hidden text, hyphenation, a quarter-turned page, `/UserUnit`, determinism, jobs and cancellation, page selection and the blocks limit, on files built with `laminapdf-build`.
- `tests/regressions.rs`: the `extract` fuzz target's entry point over the corpus and every saved crash input.
- `tests/tables.rs`: ruled grids, merged cells, a table split across two pages, a page border and field boxes (no table), ambiguous merges, unruled columns, the cell limit, and a table in Markdown.
- `tests/output.rs` (feature `schema`): Markdown goldens for the reading-order corpus (`fixtures/extract/markdown/`), the committed schema against the Rust types, every file's JSON against the schema, text ranges and chunks, and hidden text in no output. `UPDATE_GOLDEN=1` rewrites the goldens and the schema.
