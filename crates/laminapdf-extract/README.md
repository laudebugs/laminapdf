# laminapdf-extract

`laminapdf-extract` turns a PDF's text into blocks a retrieval pipeline or an LLM can use: headings, list items and paragraphs in reading order, each with a box in top-left points it can cite, and no text a reader can't see. It is part of Lamina PDF's pure core (bytes in, bytes out, no models inside it), and it implements the `laminapdf.extract/1` output that [ADR 0030](../../docs/decisions/0030-extraction-output-and-model-plugins.md) defines.

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

## Pages that need OCR

Extraction reads the text a page draws with its fonts, so a scan gives nothing, and a page made searchable by OCR gives nothing either: its OCR layer is invisible text, which extraction leaves out. Each page's `text_layer` (`textLayer` in the JSON) says which case a page is, so a pipeline can send only those pages to OCR or a vision model. It holds a status and the measures behind it: the visible characters, the invisible ones, the share of glyphs without Unicode, the images and the share of the page they cover (on a 32 × 32 grid), and the painted paths. The first status that applies wins:

| Status | When | What to do with the page |
| --- | --- | --- |
| `empty` | no text, no image and fewer than 16 painted paths | nothing: it is blank |
| `ocr-layer` | images cover at least half the page, and invisible characters outnumber visible ones | OCR it again, or keep its OCR layer (`DropOptions::keep_spans` lists it under `dropped`) |
| `needs-ocr` | no visible text; images cover at least half the page and it has at most 200 visible characters; or at least half the glyphs have no Unicode | OCR or a vision model |
| `mixed` | at least 10% of the glyphs have no Unicode, or images cover at least a quarter of the page | use the text, but parts of the page may be pictures |
| `native` | anything else | use the text |

Text left out as off the page or tiny still counts as text for `empty`, so such a page reads as `needs-ocr`, not blank. `needs-ocr` and `ocr-layer` pages also raise `extract-needs-ocr`, so a caller that reads only the Markdown learns why a page is empty. With an OCR engine set (`Models::with_ocr`), those are the pages it reads, and they raise nothing: `mixed` pages keep their own text, and `empty` pages have nothing to read. The cut-offs are starting values in `TextLayerThresholds` (`ExtractOptions::with_text_layer`), with shares in thousandths; the JSON carries every measure, so a pipeline can apply its own.

On the benchmark's 672 readable pages (below), 496 are `native`, 67 `mixed`, 67 `needs-ocr`, 42 `ocr-layer` and none `empty`. Every page that gave no text is `needs-ocr` (40) or `ocr-layer` (37). 17 of the `needs-ocr` pages still passed the benchmark's baseline test: they gave a little text, such as a stamp or glyphs without Unicode.

## How it reads a page

1. **Runs.** `laminapdf-text` gives the page's runs of glyphs with their boxes. Extraction leaves out any run that is drawn invisibly (text rendering mode 3 or 7), lies wholly off the page, or is smaller than 1 pt. It also leaves out runs wholly outside their clip (`clipped`), near-white runs with nothing darker painted where they sit (`low-contrast`), and runs the file marks as an `Artifact`: page furniture such as headers, page numbers and decoration. Text painted over later by an opaque filled rectangle, an opaque image or an annotation's appearance is `covered`: a black box drawn over two words of a line takes those words out and keeps the rest of the line. The clip and colour checks are approximations that lean towards keeping text: the clip counts as its bounding box, and any fill, stroke, image or shading that overlaps the text, drawn before or after it, counts as a backdrop unless it is near white. Each such run gets a `dropped` entry with its reason, its box and a character count, never its text. Ask for the hidden runs themselves with `DropOptions::keep_spans`.
2. **Lines.** Runs whose baselines lie within half a font size join one line, in visual order, right to left when most of the line's letters are. A gap wider than one font size between two runs on one baseline is a column gutter, so it splits them into two lines.
   The exception is a list's bullet or number: up to four font sizes from its text, it stays on the text's line.
3. **Blocks.** A line joins the block above when it follows at most 1.6 font sizes lower, at a size within 20%, overlapping the block's last line by at least 30% of the narrower of the two. A line that starts with a bullet starts a block, and so does one that starts with a dash or a number when the block above is a list item or ends in a colon.
4. **Order.** On a tagged file, the structure tree comes first (see below). Otherwise XY-Cut++ orders the blocks. Blocks that span two columns (titles, abstracts, full-width figures) split the page into bands. Each band is cut at its widest gaps, columns first, so a two-column page reads column by column; a grid of one-line blocks (a table) reads row by row; right-to-left pages take their columns from the right. `ReadingOrder::ContentStream` keeps the order the page draws instead.
5. **Text.** A block's lines are joined with spaces. A word hyphenated before a lowercase line is joined (`extrac-` / `tion`), while a hyphen before anything else stays (`well-` / `Known`). Latin ligatures are spelled out unless `/ActualText` gave the text.

On a tagged file (`/MarkInfo /Marked true`), each page whose structure tree is plausible is read in the tree's order instead of steps 2 to 4. Plausible means the tree's elements cover at least 80% of the page's visible text, and its order jumps back up into an earlier column on at most one move in ten.

Each block-level element becomes one block, and its type gives the block's kind (custom types go through the file's `RoleMap`):

- `H1` to `H6` are headings of that level; `H` and `Title` are level 1.
- `LI` is a list item whose marker is its `Lbl`; nested `L`s give the depth.
- `Table` is a table whose `TR`, `TH` and `TD` give its rows, cells, header cells and spans.
- `P`, `Caption` and `Note` are paragraphs, captions and footnotes.

Such blocks carry `structPath`, `structRef` and a `confidence` of 1. `structSource` is `tagged` when every extracted page with tagged content was read in the tree's order. Text that is neither in the tree nor an artifact comes after the tree's blocks, in layout order, and raises the `extract-untagged-content` warning. A page whose tree fails a check is read by the layout, with `extract-struct-tree-ignored`; `ReadingOrder::Tagged` uses the tree anyway, and `ReadingOrder::Layout` never does.

Once every page is read, three more rules run. The first two need the whole document: the repeated lines and the body size.

- **Running headers, footers and page numbers.** A block wholly in the top or bottom 10% of its page is a running header or footer when the same text, digits aside, sits at about the same height on at least three pages. A block there that holds only a page number (`12`, `xiv`, `- 12 -`, `Page 3 of 40`) is a page number; a bare number from 1900 to 2099 is taken for a year and kept. Each moves to `dropped` with its reason. Turning `DropOptions::furniture` off keeps them, and artifact-marked text, in the blocks.
- **Headings.** The body size is the size that sets the most characters. A block of at most three lines and 200 characters, in a size at least 10% above the body size, is a heading. The larger its size, the higher its level, and sizes within 10% of each other share a level. A bold block of one or two lines at the body size, not ending like a sentence or starting with a list marker, ranks one level below the smallest heading size. There are at most six levels, and a block that starts with a bullet is never a heading.
- **List items.** A block that starts with a bullet (`•`, `◦`, `▪`, `–`, `*`, `●`, `○`, `■`) or a number (`1.`, `1)`, `(1)`, `a.`, `(a)`, `iv.`) followed by text is a list item when its later lines hang to the right of the marker. A one-line block after a dash or an asterisk is an item only beside another item, so a lone attribution (`– Mark Twain`) stays a paragraph. The block's text leaves the marker out, and its `marker` holds it; plain text writes it back. Consecutive items each further right than the one before go one level deeper; an item further left, or one higher on the page than the last (as at the top of the next column), closes the deeper levels.

On the reading-order corpus in `fixtures/reading-order/`, 22 of the 28 files read in order, including the copies drawn in reverse: one, two and three columns, an academic page, a sidebar, a table, a rotated page, and Arabic, Hebrew and Chinese text. The other 6 are hard layouts the rules get wrong, and each raises `extract-column-order-uncertain`: a quote boxed over the gutter with the columns wrapping round it, two columns 7 pt apart, and an unruled grid with one cell on two lines.

The rules check their own column order on every page they order by layout, and raise `extract-column-order-uncertain` once for the page when a check fires. The warning's `reason` names the first check that fired, and the blocks it covers get a `confidence` of 0.5 instead of 0.75; the order itself stays the same.

- **`narrow-gutter`.** A gap between 0.6 and 1 font size is narrower than the gutter that splits a line, so the line keeps both sides. When at least three lines of a block, and at least half of them, keep such a gap within half a font size of the same place, the block has probably been read across a narrow gutter.
- **`columns-revisited`.** Within a band, the order leaves a block for one beside it at the same height, then comes back, below the pair, to both columns, each time to a block about as wide as the one above it. A column-first reading never does that; rows forced by a block across the gutter do. A row of authors over the columns, or an equation and its number, doesn't count, nor does a pair of one-line blocks, since a grid of them reads row by row on purpose.
- **`near-grid`.** A group of at least 4 blocks with gaps both ways, three quarters or more of them one line but not all, reads by columns. The rules cut it by rows as well, at most 8 times a page, and warn when the two orders differ.

## How it scores on a public benchmark

On [olmOCR-bench](https://huggingface.co/datasets/allenai/olmOCR-bench), the rules pass half of the reading-order tests, three in five of the header and footer tests, and a quarter of the table tests. The benchmark's own tests scored Lamina PDF 0.2.0-alpha.2 on 2026-11-05, on the three subsets that aren't made of scans. The data is revision `54a96a6fb6a2` of the dataset, by the Allen Institute for AI, under ODC-BY-1.0:

| Subset | What it measures | Pages | Tests | Ruled tables (the default) | Also tables without lines |
| --- | --- | ---: | ---: | ---: | ---: |
| `multi_column` | one passage before another, on multi-column pages | 231 | 884 | 52.1 % | 14.8 % |
| `table_tests` | a table cell's neighbours and headings | 188 | 1,022 | 24.1 % | 47.1 % |
| `headers_footers` | running headers, footers and page numbers left out | 266 | 760 | 59.6 % | 59.3 % |
| baseline | the page gave text (one test per page, added by the benchmark) | | 676 | 85.7 % | 85.7 % |
| **Mean, as the benchmark reports it** | | | | **55.4 %** | **51.7 %** |
| Mean of the three subsets | | | | 45.3 % | 40.4 % |

Three things explain most of the failures:

- **Pages with no text.** 89 of the 685 pages give no text. Most are scans, which need OCR; they stay in these subsets and count against the score, as they would for any user. The other 13 store their content with a filter this crate can't decode yet (ASCII85 or LZW), and one such page stops the whole extraction.
- **Single-page files.** Each of the benchmark's files is one page, so the running-header rule, which needs three pages, never applies. Dropping page numbers and artifact-marked text lifted the third subset from 37.6 %, before these rules, to 59.6 %; a header or footer that is neither stays in the text.
- **Tables without lines.** Most of the benchmark's tables have no ruling lines, so the default finds few of them. Looking for text in columns doubles the table score but reads multi-column pages as tables, which wrecks their reading order. That trade is why it stays off by default.

The column-order checks warned 48 of the 228 `multi_column` pages that extracted (Lamina PDF 0.2.0-alpha.3, 2026-11-08). The warned pages pass 48 % of their order tests and the others 54 %. The narrow-gutter pages, where the lines read across both columns, pass 9 %. The near-grid pages pass more than the rest, 64 %: most are unruled tables read column by column, which the benchmark's order tests, made from the prose, rarely check.

| Reason | Pages | Order tests | Passed |
| --- | ---: | ---: | ---: |
| `columns-revisited` | 31 | 107 | 49.5 % |
| `narrow-gutter` | 5 | 23 | 8.7 % |
| `near-grid` | 12 | 44 | 63.6 % |
| not warned | 180 | 698 | 54.2 % |

The benchmark's published table scores OCR and vision models on all seven subsets, scans and equations included, so its numbers don't compare with these. `cargo xtask extract-bench` reruns this table; [`tools/bench/`](../../tools/bench/) says how, and why the other datasets weren't used.

## Using models

The rules read most pages well. A layout model helps where they fail, on magazines, sidebars and columns set too close together; an OCR engine reads scanned pages, which have no text to extract. Neither lives in this crate. A model plugs in through a trait, so the core stays free of weights and native runtimes ([ADR 0030](../../docs/decisions/0030-extraction-output-and-model-plugins.md) §6).

- **`LayoutModel`** finds a page's regions: text, titles, section headers, list items, tables, pictures, captions, footnotes, formulas, page headers and footers. Each region then holds its own blocks, the regions set the reading order, and their labels set the block kinds. Page headers and footers leave the text as furniture. A table region with no lines in it gets the unruled table detector.
- **`OcrEngine`** reads the pages with no visible text. Its words become blocks like the text layer's, and each span records the engine's confidence in `ocr`.

A model reads a `PageRaster`, an RGBA image of the page that a `PageRenderer` you supply draws. Hand the model and the renderer to `ExtractOptions::with_models`, and the extraction calls them between pages. When your model is asynchronous (a Swift or Kotlin API, a browser worker), name it with `ModelSlot::Host` instead. Then step the job yourself: `ExtractJob::request` says which page it needs and at what resolution, and `ExtractJob::provide` hands back the result.

The output names every model it used in `generator.models` (name, version, weights' SHA-256, licence), and `generator.mode` reads `rules+models`.

| Model | What it does | Licence | Crate |
| --- | --- | --- | --- |
| Docling's Heron (RT-DETRv2), ONNX export | layout regions | weights Apache-2.0; code MIT and Apache-2.0 | [`laminapdf-layout-onnx`](../laminapdf-layout-onnx/README.md) |

Only models whose code and weights both have permissive licences may be listed; [`tools/models/models.toml`](../../tools/models/models.toml) holds them and `cargo xtask models check` enforces it.

## What it can't do yet

- **Hidden text only rendering would find.** Text inside a curved clip's bounding box counts as visible, light text over an image counts as visible even where the part of the image under it is light too, and text under a later stroke, shading, curve or rotated shape stays in the text, as does text under a box in optional content.
- **Trees that pass the checks but are wrong.** A tree that covers the text and keeps its columns in order is trusted, even where its order is still odd; extract with `ReadingOrder::Layout` to ignore it.
- **Headings and lists the rules can't see.** A heading set at the body size in a regular face reads as a paragraph, and a numbered heading in bold at the body size (`1. Introduction`) reads as a list item, not a heading. Uppercase letters (`A.`) aren't list markers, so a line that starts with an initial doesn't start a list, and uppercase Roman numerals aren't page numbers.
- **Headers that change.** A running header that changes from page to page, other than its digits, stays in the text.
- **Hard layouts.** Rules fail on magazines, floating figures and text that wraps around shapes. Paragraphs without extra space between them merge into one block. Every block carries a `confidence` below 1 to say so, and a page whose column order the checks above doubt raises `extract-column-order-uncertain`. A layout model ([Using models](#using-models)) is the answer for those pages. Gutters narrower than 0.6 font sizes look like word spaces, so the checks miss them.
- **Vertical writing.** Each vertical run is a block of its own, and a page that mixes vertical and horizontal writing raises `extract-reading-order-ambiguous`.
- **Tables without lines.** Unruled tables need `TableMode::LatticeAndStream`.

## Limits

`ExtractLimits` bounds the work per page: `max_blocks_per_page` (10,000; a page with more runs of text keeps its first runs, with `extract-limit-blocks`, before any grouping), `max_xycut_depth` (64; a deeper group reads top to bottom) and `max_table_cells` (10,000; a grid with more cells, or a page whose rules would take more than 16 times as many comparisons, gives no table, with `extract-table-limit`).

## Tests

- `tests/reading_order.rs`: every file of the reading-order corpus but the hard ones reads in order by layout, content order follows the drawing, and each hard file, but no other, raises `extract-column-order-uncertain` with its reason.
- `tests/extract.rs`: hidden text, hyphenation, a quarter-turned page, `/UserUnit`, determinism, jobs and cancellation, page selection and the blocks limit, on files built with `laminapdf-build`.
- `tests/regressions.rs`: the `extract` fuzz target's entry point over the corpus and every saved crash input.
- `tests/tables.rs`: ruled grids, merged cells, a table split across two pages, a page border and field boxes (no table), ambiguous merges, unruled columns, the cell limit, and a table in Markdown.
- `tests/text_layer.rs`: each status on pages built for it (text, a scan, a scan with an OCR layer or a stamp, text beside a picture, a blank page, shapes, glyphs without Unicode), the warning's pages, a cut-off from the options, and every page of the reading-order corpus and the text goldens `native`.
- `tests/output.rs` (feature `schema`): Markdown goldens for the reading-order corpus (`fixtures/extract/markdown/`), the committed schema against the Rust types, every file's JSON against the schema, text ranges and chunks, and hidden text in no output. `UPDATE_GOLDEN=1` rewrites the goldens and the schema.
