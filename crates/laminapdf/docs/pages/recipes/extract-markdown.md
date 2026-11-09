---
title: "Extract text for an LLM"
description: "Turn a PDF into Markdown in reading order, JSON with a box for every block, and chunks sized for retrieval, in TypeScript and Rust."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/recipes/extract-markdown/"
---

# Extract text for an LLM

Turn a PDF into Markdown in reading order, JSON with a box for every block, and chunks sized for retrieval, in TypeScript and Rust.

You'll turn a PDF into three outputs an LLM or a search index can use: Markdown in reading order, JSON giving every paragraph and table its page and box, and 400-token chunks.

In TypeScript the function returns the three outputs and needs the extract module, `init({ extract: true })`. In Rust, the program instead extracts once and writes the three files. Both give the same JSON:

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
//! cargo run -p laminapdf --example extract_markdown -- <file.pdf> [out-dir]
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

Run the Rust program on any PDF: `cargo run -p laminapdf --example extract_markdown -- report.pdf out`.

## How it works

*Extraction joins a page's text runs into lines and blocks, orders them, then writes Markdown, JSON and chunks.*

Extraction starts from each page's text runs. It sets aside runs a reader can't see, and runs the file marks as artifacts, into `dropped` with their reason: `invisible`, `off-page`, `tiny`, `clipped`, `low-contrast`, `covered` or `artifact`. The other runs are joined into lines, and the lines into blocks: paragraphs, headings and tables. Once every page is read, it also sets aside headers, footers and page numbers that repeat across pages, as `running-header`, `running-footer` or `page-number`. Reading order then puts the blocks in sequence: from the file's structure tree when it is tagged and the tree covers the page's text, else by cutting the page into columns. The ordered blocks become the three outputs: Markdown, the `laminapdf.extract/1` JSON and chunks.

*On a two-column page, a title across both columns comes first, then the left column, then the right.*

A page has a title across both columns and three blocks in each column. Extraction numbers the title 1, the left column's blocks 2 to 4 from top to bottom, and the right column's blocks 5 to 7.

- `extract` (`doc.extract` in TypeScript) reads every page's text, joins it into lines and paragraphs, and puts the paragraphs in reading order. A two-column page reads down the left column, then the right; a title across both columns comes first.
- Each block has a `confidence` from 0 to 1. When the page's columns could be read in more than one order, the page raises the `extract-column-order-uncertain` warning and the blocks in doubt get a confidence of 0.5. A pipeline can check those pages or send them to a layout model.
- Tables drawn with lines become table blocks with their cells, and Markdown writes them as tables. A cell that spans columns or rows makes the table HTML, since Markdown tables can't span.
- A table that goes on over a page break comes out as one table in the Markdown, the text and the chunks, with its header once. The JSON keeps one block per page, linked by `continuedBy` and `continues`, so each block's box stays on its own page.
- Text a reader can't see never reaches the outputs: invisible text (such as the hidden layer of a scanned page), text off the page or too small to read, and text cut away by a clipping path. Nor does near-white text with nothing behind it, or text painted over by a later shape, image or comment. Headers, footers and page numbers that repeat across pages, and text the file marks as an artifact, are left out too. The JSON lists each such run under `dropped`, with the reason and how many characters it held.
- `to_markdown` (`format: "markdown"` in TypeScript) writes headings, paragraphs and tables, with a `<!-- page N -->` comment before each page, so a passage can be traced to its page.
- `chunks` (`format: "chunks"`, or the `chunk` option with the JSON) groups whole paragraphs and tables up to the budget, starting a new chunk at each heading. Each chunk records the blocks, pages and boxes it came from, and where its text sits in the Markdown.
- `extract_json` (`format: "json"`) writes the `laminapdf.extract/1` document: the pages, every block with its box in points from the top-left corner, the dropped text, the file's warnings and the chunks.

## What to change

- **Only some pages:** `pages: [0, 1]` in TypeScript, `ExtractOptions::new().with_pages(vec![0, 1])` in Rust (page indexes start at 0).
- **Tables without lines:** `tables: "lattice-and-stream"` (`.with_tables(TableMode::LatticeAndStream)`) also looks for text lined up in columns. It finds more tables, and now and then a pair of columns that isn't one.
- **Bigger or smaller chunks:** change `maxTokens` (`with_max_tokens`). A token here is about four characters, a rough count that runs low for code, numbers and scripts other than Latin, so leave room.
- **Overlap between chunks:** `overlap: 80` (`.with_overlap(80)`) repeats up to about 80 tokens of whole paragraphs from the end of the previous chunk.
- **One output (TypeScript):** set `format` to `"markdown"`, `"text"`, `"json"` or `"chunks"`.
- **Pages that need OCR:** a scanned page has no text to extract, so its Markdown is empty. Each page in the JSON has a `textLayer` whose `status` says so: `pages.filter(p => p.textLayer.status === "needs-ocr")` in TypeScript, `extraction.pages.iter().filter(|p| p.text_layer.status == TextLayerStatus::NeedsOcr)` in Rust. Send those pages to OCR or a vision model. A page made searchable by OCR is `ocr-layer`, since extraction leaves its invisible text out; the file's warnings name both kinds.
- **A long file in steps (Rust):** `doc.extract_job(&options)` and `step` do a page per unit of work, so a long file can report progress or stop between steps.

[Benchmarks](../../pages/reference/benchmarks.md) has extraction's scores on a public benchmark and how many pages a second it reads.

Next: [all recipes](../../pages/recipes.md).
