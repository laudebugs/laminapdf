# Spike: `flow-layout` (T4a)

- **Date:** 2026-11-08
- **Status:** done. Its recommendation is recorded in [ADR 0032](../../docs/decisions/0032-flowing-layout-and-document-inputs.md), which stays Proposed until the owner signs it off.
- **Todo:** [T4a](../../docs/todos/ongoing/T4a-adr-0032-layout-engine.md); plan [trust and proof §T4](../../docs/plans/trust-and-proof.md#t4-pages-that-flow)
- **Code:** throwaway quality, outside both workspaces, never imported by product code ([code organisation §12](../../docs/guidelines/code-organization.md#12-spikes-vs-product-code)).

## Question

How should Lamina PDF lay out pages that flow: with an engine of its own over `laminapdf-build`'s `Surface`, with Typst embedded, or with a hybrid that draws Typst's layout through `Surface`? Each route lays out the same two documents here, and the measurements cover size, speed, tagging, shaping, determinism, the object graph and licences.

## Answer in one paragraph

**Build our own engine over `Surface` (route 1), and treat Typst as the reference to measure it against.** With its caches warm, Typst embedded lays the report out in 63 ms against about 190 ms for one pass of ours, and it tags better than our builder does today. But it adds 6.9 MB of gzipped WebAssembly to a 0.82 MB module, 147 crates (thread pools among them), an API that broke twice between 0.14 and 0.15, and finished bytes that Lamina PDF can only edit after reopening them. It also draws glyphs that no font has as blank boxes without a warning, and pdf.js reads its Arabic and Devanagari back scrambled. Our own paginator, 200 lines over today's builder, added 17 KB, wrote byte-identical files natively and in WebAssembly, and was read back exactly by pdf.js. What it lacked is a short list of builder features: tagged elements that continue on the next page, a way to measure text without drawing it, artifact subtypes, and a glyph-run API. Those features are the work T4b and T4c take on.

## Method

One standalone Cargo project (its own `[workspace]` table and `Cargo.lock`) with path dependencies on `crates/laminapdf`, `laminapdf-build`, `laminapdf-text`, `laminapdf-model` and `laminapdf-cos`, and `typst`, `typst-layout` and `typst-pdf` pinned at `=0.15.1`, the current release on 2026-11-08.

Two documents, the same words for every engine (`src/inputs.rs`):

- **An invoice** from `inputs/invoice.json`: a heading, the parties, 40 line items and the total, on two A4 pages. Typst reads the JSON itself through `inputs/invoice.typ` and `json()`.
- **A report** from a seeded generator: 32 sections of headings and paragraphs, 10 tables of 60 rows that cross pages, one Arabic and one Devanagari paragraph, a running header and a "Page X of Y" footer. `to_typst` writes the same content as Typst markup.

The fonts are the Noto subsets in `fixtures/fonts/` (OFL 1.1). The Devanagari subset covers only its test sentence, so the report's Devanagari paragraph repeats that sentence.

| Experiment | What it does |
| --- | --- |
| **A** own engine (`src/own.rs`) | A paginator over `Surface`: `text_box` with overflow carries paragraphs onto the next page, a heading keeps two lines of what follows, no paragraph starts on a page's last line, tables repeat their header row, and the header and footer are pagination artifacts. Because a finished page can't be reopened, a first pass counts the pages and a second draws "of Y". Everything is tagged (H1, H2, P, Table, TR, TH with a scope, TD). |
| **B** Typst embedded (`src/typst_pdf.rs`, `src/typst_world.rs`) | An in-memory `World` (sources, the fonts, a fixed date), `typst::compile`, then `typst_pdf::pdf` with a fixed identifier and timestamp and PDF/UA-1. Lamina PDF then opens the bytes, sets the title, saves incrementally and reopens the result. |
| **C** hybrid (`src/typst_frames.rs`) | Typst's laid-out frames (groups, text, shapes) drawn through `Surface` without `typst-pdf`. It stops after each document's first page, the todo's time box. |

`src/probe.rs` reads each output's structure tree back. `wasm/` builds each experiment as a `wasm32-unknown-unknown` cdylib with the root workspace's `wasm-release` profile, and `scripts/wasm.sh` runs binaryen version_133's `wasm-opt` as `cargo xtask wasm` does (`-O1`, then stripped). `scripts/node-time.mjs` times those modules in Node and hashes their output.

Typst 0.15.1 needs `skrifa` 0.42.1, through hayro and vello_cpu, and the workspace pins `=0.42.0`. `scripts/run.sh` moves that pin for one command and puts the root `Cargo.toml` back afterwards, so run everything through it:

```sh
scripts/run.sh cargo run --release                   # A, B and C; PDFs in target/flow-layout/
scripts/run.sh scripts/wasm.sh                       # WebAssembly sizes
node scripts/node-time.mjs target/wasm/own.wasm      # Node timings (after wasm.sh)
```

## Measurements

One machine: a Linux x86_64 cloud container (Intel Xeon at 2.10 GHz, 4 cores), Rust 1.99.1, `--release`, Node 24.22.1. Compare the ratios between routes rather than the absolute times, which a laptop beats.

### Size (WebAssembly, gzip -9)

| Module | raw | gzip | Added to the baseline |
| --- | ---: | ---: | ---: |
| Baseline: `laminapdf` with text, one text box saved | 2,068,438 B | 819,004 B | — |
| A: own paginator | 2,108,836 B | 836,173 B | **+17 KB** |
| B: Typst with `typst-pdf` | 19,954,965 B | 7,728,973 B | **+6.9 MB** |
| C: Typst's layout drawn through `Surface` | 19,879,955 B | 7,751,064 B | +6.9 MB |
| `pulldown-cmark` 0.13 alone (for T4d) | 179,174 B | 77,604 B | — |
| `minijinja` 2.12 alone (for T4e) | 420,410 B | 170,230 B | — |

The text module's budget is 1,130,000 B gzip ([ADR 0016 §6](../../docs/decisions/0016-package-design.md#6-budgets-per-module)). Typst's module splits into 3.8 MB of gzipped code and 3.9 MB of gzipped data. Leaving out `typst-pdf` (C) saves nothing, because the layout engine and what it bundles carry the weight.

### Speed

| | Invoice (2 pages) | Report (A: 89 pages, B: 97 pages) |
| --- | ---: | ---: |
| A native, both passes, best of 30 / 10 | 6.6 ms | 379 ms |
| B native, cold (cache evicted), best of 30 / 10 | 10.4 ms | 396 ms |
| B native, warm cache | 1.4 ms | 63 ms |
| A in Node 24, first run / best | 85 ms / 10.8 ms | 1,046 ms / 555 ms |
| B in Node 24, first run / best | 153 ms / 5.4 ms | 1,259 ms / 81 ms |

The report times include A's two passes. A also draws every line twice, once to place it and once to fill in "of Y", so one pass costs about half. B's warm figure is what a long-running service would see: Typst memoises across compiles with `comemo`, and its report took 138 MiB of WebAssembly memory against A's 10 MiB. Against [pdf-library §4](../../docs/plans/pdf-library.md#4-requirements)'s 500 ms for a 100-page report, both routes need work in WebAssembly: A by laying out once, B by being warm.

### Tagged output

| | A (own engine) | B (Typst) |
| --- | --- | --- |
| veraPDF PDF/UA-1, invoice | passes | passes |
| veraPDF PDF/UA-1, report | the builder refuses it: `glyph-without-unicode` on an Arabic mark | passes |
| A paragraph across a page break | two `P` elements (37 splits) | one `P` (41 paragraphs) |
| A table across a page break | a `Table` per page (23 for 10 tables) | one `Table` (10), with `THead` and `TBody` |
| Running header and footer | `/Artifact` `/Pagination`, no subtype | `/Pagination` with `/Subtype /Header` or `/Footer` and `/Attached` |

Both engines mark the header and footer as artifacts, and Lamina PDF's extraction drops them from both. The Arabic sentence has diacritics. With PDF/UA-1 on, Typst writes a Unicode value for every glyph, while our builder gives a mark that shares a cluster with its letter no value of its own and refuses the file. Without PDF/UA-1, Typst's report failed veraPDF's rule 7.21.7 (character codes without Unicode) 14 times.

### Shaping and extraction

| Read back by | A | B | C |
| --- | --- | --- | --- |
| pdf.js 6.4.299: Arabic / Devanagari equal to the input | yes / yes | no / no | no / yes |
| pdftotext 24.02: Arabic / Devanagari | no / yes | no / yes | no / yes |

Whitespace and bidi controls are removed before comparing. pdftotext splits A's Arabic into three lines and misplaces one shadda, so it reads none of the three Arabic paragraphs back. `hb-shape` wasn't installed, so the glyph-level comparison with `fixtures/shaping/` wasn't run. A shapes with `laminapdf-text`, which matches those goldens already, and Typst shapes with rustybuzz 0.20.

Text whose glyphs no font has: our builder fails with `missing-glyph`. Typst compiles it with no error and no warning, and the page shows blank boxes.

### Determinism

Every output was byte-identical across 30 runs, natively. The WebAssembly build wrote the same bytes as the native one (same SHA-256) for A's report and both of B's documents.

### Object graph

- **A and C** draw into a `Builder`, so their pages live in a Lamina PDF document's overlay: generate, then edit and save, with undo.
- **B** gives finished bytes. Lamina PDF opens them and saves an edit incrementally (+348 B for a new title), but its pages can't join an open document, because Lamina PDF has no page import. The [`krilla-fit`](../krilla-fit/README.md) spike found the same for krilla, on which `typst-pdf` writes: its imported pages lost their tags.

### What experiment C couldn't map

`Surface` draws strings, not glyph runs, so C had to shape Typst's text again with our shaper. Latin came out where Typst put it. Typst splits an Arabic line into many short items, though, and shaping each one alone broke the joining: 65 of the report page's 122 text items came out more than 0.5 pt wider or narrower than Typst's. Typst's 346 introspection tags across the two pages couldn't be used either, because its tagger lives in `typst-pdf`. A working hybrid needs a glyph-run API on `Surface` and a tagger of our own, which is most of route 1's work plus Typst's size.

### Licences and upkeep

- `cargo deny --manifest-path Cargo.toml --config ../../deny.toml check licenses bans sources` passes: every licence is in the allow-list, with 8 crates in two versions each. Typst's bundled fonts sit behind `typst-assets`' `fonts` feature, which is off.
- The spike's graph has 323 crates, 147 of which the workspace lacks. Among them are a WebAssembly runtime (`wasmi`), syntax highlighting (`syntect`), bibliographies (`hayagriva`, `biblatex`, `citationberg`), SVG (`resvg`, `usvg`), image decoding, hyphenation (`hypher`), a second shaper (`rustybuzz`), thread pools (`rayon`, `parking_lot`) and stack growth (`stacker`, `psm`). The threads alone keep it out of a core crate.
- Typst's minor releases came 4 to 8 months apart: 0.11 in March 2024, 0.12 in October 2024, 0.13 in February 2025, 0.14 in October 2025 and 0.15 in June 2026. Between 0.14 and 0.15 the spike met two breaks: `FileId::new(None, VirtualPath)` became `RootedPath::new(VirtualRoot::Project, …).intern()`, and `PagedDocument` moved from `typst::layout` to the new `typst-layout` crate. File IDs are interned for the life of the process and capped at 65,535.

## What our own engine lacked

Experiment A ran on today's builder. These gaps shaped its output, and each one is work for T4b or T4c:

1. **Elements across pages.** Tagged elements end with their page, so a paragraph or table split by a page break becomes two elements.
2. **Measuring without drawing.** Nothing lays text out without drawing it. Keep rules have to assume a line height, and table columns can't be sized to their content: a cell too narrow clips silently, as the invoice's descriptions did until the spike gave the first column 40 %.
3. **Deferred content.** A finished page can't be reopened, so "Page X of Y" costs a second pass.
4. **Artifact subtypes.** `ArtifactKind::Pagination` writes no `/Subtype` (Header, Footer, Watermark).
5. **Marks under PDF/UA-1.** A mark that shares a cluster with its letter gets no Unicode value, so PDF/UA-1 refuses Arabic with diacritics.
6. **A glyph-run API.** `Surface` draws strings; drawing shaped glyphs as they come would serve a hybrid and fallback fonts alike.

## Recommendation

Make flowing layout a library goal and build it ourselves over `Surface`, starting with the six gaps above in the builder. Embed Typst neither as the engine nor as a front end: its size alone is six times the text module's whole budget, and it would tie the core to a 0.x API that breaks between minor releases. Keep the Typst comparison as a test: the same documents laid out by both, with Typst's tagging (one element across pages, header and footer subtypes) as the bar to meet.
