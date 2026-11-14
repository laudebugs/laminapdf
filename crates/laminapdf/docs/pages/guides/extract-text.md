---
title: "Extract text and Markdown"
description: "Get a PDF's text out in reading order as plain text, Markdown, JSON with a box for every block, or chunks for retrieval, in TypeScript and Rust."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/extract-text/"
---

# Extract text and Markdown

Get a PDF's text out in reading order as plain text, Markdown, JSON with a box for every block, or chunks for retrieval, in TypeScript and Rust.

You'll get a PDF's text out in the form your program needs: plain text to search, Markdown for a language model, JSON with each paragraph's page and box, or chunks for a search index. Each form writes out the same reading of the document.

In TypeScript, extraction runs in the extract module or the text module. Call `init({ extract: true })` once before you open the document, unless your app already loads the text module. In Rust, it needs the crate's `extract` feature, which the default features include.

## Get the text

`extract` with the `text` format gives the page's words in the order a person reads them. A blank line separates paragraphs and headings, and tabs separate a table's cells:

**TypeScript**

`packages/sdk/examples/extract-text.ts`:

```ts
// Read a PDF's text in reading order: paragraphs, headings and table rows, a blank line between
// blocks, and nothing a reader can't see.
import { Document } from "@laminapdf/sdk";

/** The text of `bytes`, in reading order. Needs `init({ extract: true })`. */
export function textOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  return doc.extract({ format: "text" });
}
```

**Rust**

`crates/laminapdf/examples/extract_text.rs`:

````rust
//! Read a PDF's text in reading order: paragraphs, headings and table rows, a blank line between
//! blocks, and nothing a reader can't see.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_text -- <file.pdf>
//! ```

use std::error::Error;

use laminapdf::extract::ExtractOptions;
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_text <file.pdf>")?;
    print!("{}", text_of(std::fs::read(input)?)?);
    Ok(())
}

/// The text of `bytes`, in reading order.
pub(crate) fn text_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    Ok(extraction.to_text().text)
}
````

**Output**

`fixtures/extract/examples/invoice.txt`:

```text
Invoice 42

Acme Widgets Ltd · 1 Example Street · London

Widgets, blue (W-100)	3	€ 36.00
Gadget with fine-tuned dials	1	€ 79.90
Café-grade sprockets	12	€ 15.00
Delivery	1	€ 4.50

Total € 135.40
```

A PDF stores where each piece of text is drawn, not the order to read it in. Extraction works the order out, so the text comes back the way the page reads, even when the file drew it in another order:

*Extraction joins a page's text runs into lines and blocks, orders them, then writes Markdown, JSON and chunks.*

Extraction starts from each page's text runs. It sets aside runs a reader can't see, and runs the file marks as artifacts, into `dropped` with their reason: `invisible`, `off-page`, `tiny`, `clipped`, `low-contrast`, `covered` or `artifact`. The other runs are joined into lines, and the lines into blocks: paragraphs, headings and tables. Once every page is read, it also sets aside headers, footers and page numbers that repeat across pages, as `running-header`, `running-footer` or `page-number`. Reading order then puts the blocks in sequence: from the file's structure tree when it is tagged and the tree covers the page's text, else by cutting the page into columns. The ordered blocks become the three outputs: Markdown, the `laminapdf.extract/1` JSON and chunks.

## Get Markdown

Markdown keeps what plain text loses: headings stay headings, lists stay lists, and a table drawn with lines becomes a Markdown table. That structure helps a language model tell a title from a footnote. The `pageMarkers` option (`with_page_markers` in Rust) writes a `<!-- page N -->` comment before each page, so an answer can cite its page:

**TypeScript**

`packages/sdk/examples/extract-markdown.ts`:

```ts
// Extract a PDF's text for an LLM: Markdown in reading order, JSON with a box for every block,
// and chunks sized for retrieval.
//
// Tables drawn with lines come out as Markdown tables, and text a reader can't see (white,
// invisible or off the page) is left out of all three.
import { type Chunk, Document, type Extraction } from "@laminapdf/sdk";

/** What a PDF gives an LLM or a search index. */
export interface ForAnLlm {
  /** The text in reading order, with a `<!-- page N -->` comment before each page. */
  readonly markdown: string;
  /** Every block with its page and box, the text left out, the file's warnings and the chunks. */
  readonly json: Extraction;
  /** About 400 tokens each, a new one at each heading. */
  readonly chunks: readonly Chunk[];
}

/** The Markdown, the JSON and the chunks of `bytes`. Needs `init({ extract: true })`. */
export function extractForAnLlm(bytes: Uint8Array): ForAnLlm {
  using doc = Document.open(bytes);
  // Page markers let a reader cite the page a passage came from.
  const markdown = doc.extract({ format: "markdown", pageMarkers: true });
  // The JSON carries the same chunks, and each block's place in the Markdown.
  const json = doc.extract({ format: "json", pageMarkers: true, chunk: { maxTokens: 400 } });
  return { markdown, json, chunks: json.chunks ?? [] };
}
```

**Rust**

`crates/laminapdf/examples/extract_markdown.rs`:

````rust
//! Extract a PDF's text for an LLM: Markdown in reading order, JSON with a box for every
//! block, and chunks sized for retrieval.
//!
//! Tables drawn with lines come out as Markdown tables, and text a reader can't see (white,
//! invisible or off the page) is left out of all three.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example extract_markdown -- <file.pdf> [out-dir]
//! # writes <out-dir>/<file>.md, <file>.json and <file>.chunks.jsonl
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::extract::{ChunkOptions, ExtractOptions, MarkdownOptions, chunks_jsonl};
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: extract_markdown <file.pdf> [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    write_outputs(&std::fs::read(&input)?, &out, stem)
}

/// Writes the Markdown, the JSON and the chunks of `bytes` into `out`.
pub(crate) fn write_outputs(bytes: &[u8], out: &Path, stem: &str) -> Result<(), Box<dyn Error>> {
    let doc = Document::open_bytes(bytes.to_vec(), OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;

    // Page markers let a reader cite the page a passage came from.
    let markdown = extraction.to_markdown(&MarkdownOptions::new().with_page_markers(true));
    std::fs::write(out.join(format!("{stem}.md")), &markdown.text)?;

    // About 400 tokens a chunk, starting a new one at each heading.
    let chunks = extraction.chunks(&markdown, &ChunkOptions::new().with_max_tokens(400));
    std::fs::write(out.join(format!("{stem}.chunks.jsonl")), chunks_jsonl(&chunks))?;

    let json = doc.extract_json(&extraction, &markdown, Some(chunks));
    std::fs::write(out.join(format!("{stem}.json")), json)?;
    Ok(())
}
````

**Output**

`fixtures/extract/examples/invoice.md`:

```md
<!-- page 1 -->

# Invoice 42

Acme Widgets Ltd · 1 Example Street · London

| Widgets, blue (W-100) | 3 | € 36.00 |
| --- | --- | --- |
| Gadget with fine-tuned dials | 1 | € 79.90 |
| Café-grade sprockets | 12 | € 15.00 |
| Delivery | 1 | € 4.50 |

Total € 135.40
```

The example also writes the JSON and the chunks, which the next sections describe. The [Extract text for an LLM](../../pages/recipes/extract-markdown.md) recipe shows how to run it on your own files.

## Get each block with its box

The `json` format returns the whole result as data: every block with its kind, its page and its box (left, top, width and height, in points from the page's top-left corner). Use it to highlight a passage on the page, to keep a table's cells apart, or to check what extraction left out. Here each block prints as one line:

**TypeScript**

`packages/sdk/examples/extract-blocks.ts`:

```ts
// List a PDF's blocks with where they sit: each heading, paragraph, list item and table, its page
// and its box in points from the page's top-left corner.
import { Document, type ExtractedBlock } from "@laminapdf/sdk";

/**
 * One line per block: its page number, its kind, its box (left, top, width, height) and its text.
 * Needs `init({ extract: true })`.
 */
export function blocksOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  const extraction = doc.extract({ format: "json" });
  return extraction.blocks
    .map((block) => {
      const box = block.bbox.map((v) => Math.round(v)).join(" ");
      const text = block.text.replace(/[\t\n]/g, " ");
      return `page ${block.page + 1} ${kind(block)} [${box}] ${text}\n`;
    })
    .join("");
}

/** The block's kind as the JSON names it, with a heading's level. */
function kind(block: ExtractedBlock): string {
  return block.type === "heading" ? `heading ${block.level}` : block.type.replaceAll("-", " ");
}
```

**Rust**

`crates/laminapdf/examples/extract_blocks.rs`:

````rust
//! List a PDF's blocks with where they sit: each heading, paragraph, list item and table, its page
//! and its box in points from the page's top-left corner.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_blocks -- <file.pdf>
//! ```

use std::error::Error;
use std::fmt::Write as _;

use laminapdf::extract::{Block, BlockKind, ExtractOptions};
use laminapdf::{Document, OpenOptions, Pt};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_blocks <file.pdf>")?;
    print!("{}", blocks_of(std::fs::read(input)?)?);
    Ok(())
}

/// One line per block: its page number, its kind, its box (left, top, width, height) and its text.
pub(crate) fn blocks_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    let mut out = String::new();
    for block in &extraction.blocks {
        let bbox = block.bbox;
        let place = [bbox.x0, bbox.y0, bbox.width(), bbox.height()].map(|v| whole_points(v).to_string());
        let text = block.text.replace(['\t', '\n'], " ");
        writeln!(
            out,
            "page {} {} [{}] {text}",
            block.page + 1,
            kind(block),
            place.join(" ")
        )?;
    }
    Ok(out)
}

/// The block's kind as the JSON names it, with a heading's level.
fn kind(block: &Block) -> String {
    match &block.kind {
        BlockKind::Heading { level } => format!("heading {level}"),
        BlockKind::Paragraph => "paragraph".into(),
        BlockKind::ListItem { .. } => "list item".into(),
        BlockKind::Caption => "caption".into(),
        BlockKind::Footnote => "footnote".into(),
        BlockKind::Table { .. } => "table".into(),
        // `Other`, and any kind a later version adds.
        _ => "other".into(),
    }
}

/// `v` in whole points, rounded as the JSON rounds it (to a thousandth), then halves up.
fn whole_points(v: Pt) -> f64 {
    let json = (v.get() * 1000.0).round() / 1000.0;
    (json + 0.5).floor()
}
````

**Output**

`fixtures/extract/examples/invoice-blocks.txt`:

```text
page 1 heading 1 [56 63 116 22] Invoice 42
page 1 paragraph [56 88 237 15] Acme Widgets Ltd · 1 Example Street · London
page 1 table [56 148 483 87] Widgets, blue (W-100) 3 € 36.00 Gadget with fine-tuned dials 1 € 79.90 Café-grade sprockets 12 € 15.00 Delivery 1 € 4.50
page 1 paragraph [468 244 71 15] Total € 135.40
```

A block is a heading, a paragraph, a list item, a caption, a footnote or a table. A table block also holds its cells, row by row. In Rust, `doc.extract` returns the same data as Rust types, and `doc.extract_json` writes the JSON document, `laminapdf.extract/1`.

Each block also has a `confidence` from 0 to 1. A block read from the file's structure tree, the tags that give a tagged PDF its reading order, has 1. A block placed by the layout has 0.75, and 0.5 when its page's columns could be read in more than one order. With `readingOrder: "content-stream"`, every block has 0.5, since the drawing order may not be the reading order.

## Get chunks for retrieval

A search index or a retrieval pipeline wants pieces of a set size. Chunks are whole paragraphs and tables, grouped up to a token budget, with a new chunk at each heading. Each chunk records the headings above it, the pages and boxes it came from, and where its text sits in the Markdown:

**TypeScript**

`packages/sdk/examples/extract-chunks.ts`:

```ts
// Split a PDF into chunks for retrieval: whole paragraphs and tables up to a token budget, a new
// chunk at each heading, each with the headings above it and the pages it came from.
import { Document } from "@laminapdf/sdk";

/** Each chunk of `bytes`: its headings, its pages, its size and its text. Needs `init({ extract: true })`. */
export function chunksOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  // A small budget, so this short file gives several chunks; 400 to 800 suits most indexes.
  const chunks = doc.extract({ format: "chunks", chunk: { maxTokens: 60 } });
  return chunks
    .map((chunk) => {
      const pages = chunk.pages.map((p) => p + 1).join(", ");
      return (
        `${chunk.id} under "${chunk.headingPath.join(" > ")}"\n` +
        `pages ${pages}, about ${chunk.tokensApprox} tokens\n` +
        `${chunk.text.trimEnd()}\n\n`
      );
    })
    .join("");
}
```

**Rust**

`crates/laminapdf/examples/extract_chunks.rs`:

````rust
//! Split a PDF into chunks for retrieval: whole paragraphs and tables up to a token budget, a new
//! chunk at each heading, each with the headings above it and the pages it came from.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_chunks -- <file.pdf>
//! ```

use std::error::Error;
use std::fmt::Write as _;

use laminapdf::extract::{ChunkOptions, ExtractOptions, MarkdownOptions};
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_chunks <file.pdf>")?;
    print!("{}", chunks_of(std::fs::read(input)?)?);
    Ok(())
}

/// Each chunk of `bytes`: its headings, its pages, its size and its text.
pub(crate) fn chunks_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    // Chunks are cut from the Markdown, so each one keeps its headings and tables.
    let markdown = extraction.to_markdown(&MarkdownOptions::new());
    // A small budget, so this short file gives several chunks; 400 to 800 suits most indexes.
    let chunks = extraction.chunks(&markdown, &ChunkOptions::new().with_max_tokens(60));
    let mut out = String::new();
    for chunk in &chunks {
        let pages: Vec<String> = chunk.pages.iter().map(|p| (p + 1).to_string()).collect();
        writeln!(out, "{} under \"{}\"", chunk.id, chunk.heading_path.join(" > "))?;
        writeln!(out, "pages {}, about {} tokens", pages.join(", "), chunk.tokens_approx)?;
        writeln!(out, "{}\n", chunk.text.trim_end())?;
    }
    Ok(out)
}
````

**Output**

`fixtures/extract/examples/invoice-chunks.txt`:

```text
c0 under "Invoice 42"
pages 1, about 58 tokens
# Invoice 42

Acme Widgets Ltd · 1 Example Street · London

| Widgets, blue (W-100) | 3 | € 36.00 |
| --- | --- | --- |
| Gadget with fine-tuned dials | 1 | € 79.90 |
| Café-grade sprockets | 12 | € 15.00 |
| Delivery | 1 | € 4.50 |

c1 under "Invoice 42"
pages 1, about 4 tokens
Total € 135.40

```

A token here is about four characters. That count runs low for code, numbers and scripts other than Latin, so leave room under your model's limit. `overlap` repeats whole paragraphs from the end of the previous chunk, and `splitOn` (`with_split_on`) chooses where else a chunk starts: at each heading (`"headings"`, the default), at each page (`"pages"`), or only when the budget runs out (`"blocks"`).

## Reading order

Extraction reads a two-column page down the left column, then down the right, and puts a title that spans both columns first:

*On a two-column page, a title across both columns comes first, then the left column, then the right.*

A page has a title across both columns and three blocks in each column. Extraction numbers the title 1, the left column's blocks 2 to 4 from top to bottom, and the right column's blocks 5 to 7.

The `readingOrder` option (`with_reading_order` in Rust) chooses where the order comes from:

| `readingOrder` | Where the order comes from |
| --- | --- |
| `"auto"` (the default) | The structure tree of a tagged file when it covers the page and looks right, otherwise the layout |
| `"tagged"` | The structure tree wherever there is one |
| `"layout"` | The layout alone, columns then rows |
| `"content-stream"` | The order the page draws its text |

When the layout leaves the order in doubt, such as two columns set very close together, the page raises the `extract-column-order-uncertain` warning. [Troubleshooting text extraction](../../pages/troubleshooting/extraction.md) says what to do about it and about the other surprises.

## What extraction leaves out

Extraction gives you only the text a reader can see. It leaves out:

- invisible text, such as the hidden text layer of a scanned page;
- text off the page, too small to read, or cut away by a clipping path;
- near-white text with nothing behind it, and text painted over by a later shape, image or comment;
- page furniture: running headers and footers, page numbers, and text the file marks as an artifact.

The JSON lists each run it left out under `dropped`, with the reason and the number of characters, but not the text. To keep the text in that list, set `drop: { keepSpans: true }`; in Rust, set the options' `drop.keep_spans` field to `true`. To keep page furniture in the blocks, set `drop: { furniture: false }` (`drop.furniture` in Rust).

## Tables

A table drawn with lines becomes a table block, and Markdown writes it as a table. A cell that spans columns or rows makes the table HTML, since Markdown tables can't span. A table that goes on over a page break comes out as one table in the Markdown, the text and the chunks. The JSON keeps one block per page, linked by `continues` and `continuedBy`.

A table drawn without lines is only text lined up in columns. To find those too, set `tables: "lattice-and-stream"` (`with_tables(TableMode::LatticeAndStream)` in Rust). It finds more tables, and now and then a pair of columns that isn't one.

## Warnings to expect

Extraction never fails on a page it reads badly. It returns what it could read and adds a warning, which the JSON lists under `warnings`:

| Warning | What it means |
| --- | --- |
| `extract-needs-ocr` | A page has no text to use, such as a scan; send it to OCR (optical character recognition) |
| `extract-column-order-uncertain` | The page's columns could be read in more than one order |
| `extract-struct-tree-ignored` | A tagged page's structure tree failed the checks, so the layout gave the order |
| `unmapped-glyph` | A font drew glyphs that map to no character; they come out as `�` |
| `extract-table-ambiguous` | A table's cells merge in a shape a grid can't hold |

Each page in the JSON also has a `textLayer` whose `status` says whether its text is usable:

| `status` | The page |
| --- | --- |
| `native` | Text the file draws |
| `mixed` | Usable text, though some glyphs map to no character or images cover part of the page |
| `needs-ocr` | No text to use, such as a scan |
| `ocr-layer` | A scan with an invisible OCR layer, which extraction leaves out |
| `empty` | Nothing on it |

Pages that need OCR can go to a layout or OCR model through `extractWithModels` (`ExtractOptions::with_models` in Rust); without one, extraction uses its own rules alone.

Next: [Troubleshooting text extraction](../../pages/troubleshooting/extraction.md), for empty pages, columns read across and missing characters.
