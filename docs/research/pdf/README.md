# PDF research

A deep technical study of how PDF works and how to produce and process it from code. This is the foundation for lamina's design decisions.

Primary reference throughout: **ISO 32000-2:2020 (PDF 2.0)**. Section numbers ("§x.y") refer to it unless stated.

| # | Chapter | What it answers |
| --- | --- | --- |
| 01 | [File structure & syntax](01-file-structure.md) | What bytes are in a PDF? Objects, xref tables and streams, object streams, incremental updates, linearisation, filters, page tree, how parsers read (and repair) files. Includes a verified hand-written "Hello, World" PDF. |
| 02 | [The imaging model](02-imaging-model.md) | How does PDF describe what's on a page? Content-stream operators, coordinate systems and matrices, paths and Béziers, colour science and colour spaces, images, patterns and shadings, transparency, how renderers rasterise. |
| 03 | [Text, fonts, extraction & accessibility](03-text-and-fonts.md) | Why PDF stores glyphs, not text. Font types, encodings, CMaps and ToUnicode, embedding and subsetting, shaping and layout, text extraction and OCR, tagged PDF and PDF/UA. |
| 04 | [Interactive features, metadata, security & signatures](04-interactive-security.md) | Links, annotations, forms (AcroForm/XFA), JavaScript, XMP, attachments, encryption, digital signatures and PAdES, redaction, known attacks, PDF as an attack surface. |
| 05 | [Creating, processing & rendering PDFs from code](05-creating-pdfs-from-code.md) | The approaches (low-level writers to HTML→PDF to typesetting), library ecosystem per language with licences, processing operations, renderers, testing, and a recommendation for lamina. |
| 06 | [Feature inventory](06-feature-inventory.md) | Every operation a full PDF platform/editor offers (~180, in 13 groups), mapped to the PDF mechanism, difficulty and chapter. Reference architecture and phasing. |
| 07 | [Spike: Rust and TypeScript](07-spike-rust-and-typescript.md) | The same minimal PDF core built in both languages from one spec, with shared tests (byte-identical golden files, pypdf-checked corpus, fuzzing, Rust→WASM→TS interop). Results, lessons and what it means for the core-language decision. |

Runnable examples that back the chapters are in [`examples/`](examples/). Source documents (e.g. the owner's NotebookLM report) are in [`sources/`](sources/).

## Reading order

- **New to PDF internals:** 01 → 02 → 03, then 06 for the big picture.
- **Deciding what to build:** 06 → 05 (§ Implications for lamina) → the "Implications" section at the end of each chapter.

## Caveats

- Chapters were researched in October 2026. Library versions, maintenance status and licences change; re-check before depending on them.
- **Verified against ISO 32000-1:2008 (PDF 1.7)** and Adobe TN #5411 (ToUnicode) on 2026-10-28 — about 125 corrections in total. Each chapter ends with a *Verification log* listing every correction and what remains unverified.
- PDF 2.0-only claims have **not** been checked against ISO 32000-2 text (only its clause-12 errata). They're labelled "PDF 2.0" in the chapters.
