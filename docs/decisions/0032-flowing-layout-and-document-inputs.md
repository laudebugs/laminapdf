# 0032 — Flowing layout and document inputs

- Status: Accepted (2026-11-08, owner sign-off on T4a's PR)
- Date: 2026-11-08
- Amended 2026-11-10 (owner, T4i): §9's text-module budget rises to 1,360,000 B gzip, as [ADR 0016](0016-package-design.md#6-budgets-per-module)'s amendment records.
- Amended 2026-11-11 (owner, T4j): §9's text-module budget rises to 1,400,000 B gzip for JSON documents and `builder.flow`, as ADR 0016's second amendment records.

## Context

Control over page breaks is the generation request with the clearest gap on Hacker News. About 345 comments ask for page breaks, running headers, page numbers, contents pages, footnotes and repeated table headers ([report R5](../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json)). The two largest generation requests depend on it: writing Markdown (about 1,140 comments) and filling a template from JSON (about 560). Today `laminapdf-build` breaks lines inside one box and reports what overflows, and [pdf-library §2](../plans/pdf-library.md#2-scope-and-non-goals) lists a flowing layout engine as a non-goal. Its §9 asks whether layout becomes a goal and whether Typst is the route (question 4), and whether to offer a declarative layer (question 3).

Typst is the obvious candidate to embed. It went from no Hacker News mentions to 1,640 in 2023–26, 94 comments describe it running in production from data, and it is Apache-2.0. Its PDF export writes through krilla, which the [`krilla-fit` spike](../../spikes/krilla-fit/README.md) found returns finished bytes only ([ADR 0005 §6](0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges)).

The [`flow-layout` spike](../../spikes/flow-layout/README.md) laid out the same two documents three ways: a two-page invoice from JSON and a 100-page report with tables across pages, an Arabic and a Devanagari paragraph, a running header and "Page X of Y". The routes were an engine of our own over `Surface` (A), Typst 0.15.1 embedded (B), and Typst's layout drawn through `Surface` (C). Its findings decide every point below. This ADR makes flowing layout a library goal, builds it ourselves on `laminapdf-build`, and takes Markdown and a JSON document format as its inputs.

## Decision

### 1. Flowing layout is a library goal

Lamina PDF lays out documents that flow across pages: the caller hands over blocks and a page template, and the library makes as many pages as the content needs. This answers pdf-library §9 question 4 and removes the non-goal from §2. The fixed-position builder stays as it is, and the flow layer draws through it.

### 2. Our own engine over `Surface`, with Typst as the measuring stick

The flow layer is Lamina PDF's own code in `laminapdf-build`. Typst is neither embedded as the engine nor offered as a front end. The spike's numbers:

| | Own engine (A) | Typst embedded (B) |
| --- | --- | --- |
| Added to the text module (WebAssembly, gzip) | +19 KB | +7.36 MB, against a 1.13 MB budget |
| Crates added to the workspace's graph | 0 | 147, with thread pools (`rayon`) and stack growth (`stacker`, `psm`) |
| 100-page report, native | about 190 ms per pass | 386 ms cold, 62 ms with a warm cache |
| Pages in the document's overlay (generate, then edit, with undo) | yes | no: finished bytes, edited after reopening; no page import |
| Glyphs no font has | `missing-glyph` | blank boxes, no warning |
| pdf.js reads back the Arabic and Devanagari paragraphs | both | neither |
| Byte-identical output, native and WebAssembly | yes | yes |

Typst's API also broke twice between 0.14 and 0.15, and its minor releases come 4 to 8 months apart. Typst does tag better than our builder today: a paragraph or table across a page break stays one element, and headers and footers carry `/Subtype`s. Those behaviours become requirements for our engine (§4). The spike's two documents stay as the comparison: T4b's tests hold our engine to the structure Typst wrote for them.

The hybrid (C) is rejected too. `Surface` draws strings, so Typst's glyphs had to be shaped again, which broke Arabic joining, and Typst's tagger lives in `typst-pdf`. A working hybrid needs a glyph-run API and a tagger of our own, which is most of A's work with all of Typst's size.

### 3. The layout model

**Blocks:** `Paragraph` (inline spans of styled text, links and line breaks), `Heading` (levels 1–6), `List` (bulleted or numbered, nested), `Figure` (an image with alt text), `Table`, `Spacer`, `PageBreak` and `Keep` (blocks kept on one page). Footnotes and a contents page are blocks too; T4h adds them (§10).

**Pagination** is greedy and deterministic. A paragraph splits between lines with at least two lines at the foot and the head of a page (`widows` and `orphans`, 2 and 2 by default, per paragraph style). A heading keeps with the block after it; a `Keep` group and a figure move whole. Optimal (Knuth–Plass) breaking is out: it costs layout passes for a gain few documents need.

**Page templates** give the page size, margins, a first-page variant, and a header and a footer with start, centre and end slots. Slots hold text and fields: `{page}` (1-based), `{pages}`, `{section}` (the last heading of a chosen level started on or before the page) and `{title}`. [E6d](../todos/ready/E6d-watermarks-headers-footers.md), which stamps headers and footers on existing documents, uses the same fields and syntax.

**"Page X of Y" and contents pages** come from laying out before drawing. The flow measures every block, breaks every page, and only then draws, so `{pages}` is known when footers are drawn. The spike's paginator drew everything twice instead, which a builder without measuring forces; the flow layer needs a measure call (§10's groundwork). A contents page lays out again until no page number changes, at most four passes, then fails with `flow-layout-unstable`.

**Left out**, each with its home:

- floats: not planned;
- multiple columns: not planned until a request names it;
- justification: a later todo (`laminapdf-text` reserves `LayoutOptions::justify`);
- hyphenation: a later todo with justification ([research 03 §7.5](../research/pdf/03-text-and-fonts.md#75-hyphenation-knuthliang));
- maths: not planned;
- HTML and CSS: [E13f](../todos/ready/E13f-office-and-html-to-pdf.md), as a server job.

### 4. Tagging

Flowing output is tagged by default, and a caller can turn tagging off. This answers [trust and proof §9](../plans/trust-and-proof.md#9-open-questions) question 1 for flowing output; the fixed-position builder keeps its opt-in. The spike's paginator met PDF/UA-1 on the invoice with no extra work, and Typst tags by default too.

- **Mapping:** `P`, `H1`–`H6`, `L`, `LI`, `Lbl`, `LBody`, `Figure` with `/Alt`, `Link`, `Span` with `/Lang` for a span in another language, and `Table` with `THead`, `TBody`, `TFoot`, `TR`, `TH` (`/Scope /Column`) and `TD`.
- **Elements across pages:** a paragraph, list or table split by a page break stays one element, whose `/K` reaches its content on later pages through marked-content references. The spike's builder made two elements of each of 36 split paragraphs, and 24 `Table` elements for 10 tables.
- **Artifacts:** running headers and footers are `Pagination` artifacts with `/Subtype /Header` or `/Footer` and `/Attached`, as Typst writes them. Header rows repeated on a later page are `Pagination` artifacts; rules and backgrounds are `Layout` artifacts.
- **PDF/UA-1:** every glyph has a Unicode value. The spike's builder refused Arabic with diacritics: a mark that shares a cluster with its letter got no value (`glyph-without-unicode`). Typst wrote the same text and veraPDF passed it, and the groundwork todo fixes our builder.

**Implementation note (T4f):** the glyph without a value wasn't a diacritic but a letter's dot. Noto Sans Arabic draws beh, teh, noon and feh as a dotless body and a dot. The body carries the letter, and the dot maps to the empty value, so each letter is read once. veraPDF 1.30.3 passes that under PDF/UA-1 and PDF/A-2u, and pdf.js reads the text back exactly, so the builder's profile check now counts the empty value as a value. Giving the dot the letter's text as well would make pdf.js, which ignores `/ActualText`, read every dotted letter twice. Elements across pages are `Surface::open_element`, `Surface::resume` and `Builder::close_element`; the artifacts are `Artifact::header`, `footer` and `watermark`.

### 5. Placement

- **The flow layer** is a `flow` module of `laminapdf-build`, not a crate of its own. It shares `Surface`, the font registry and the tagger, and splitting it would expose their internals ([code organisation §11](../guidelines/code-organization.md#11-when-to-split-a-crate-or-package-and-when-not-to)). The façade re-exports it as `laminapdf::flow`.
- **The Markdown front end** is a `markdown` feature of `laminapdf-build`, over `pulldown-cmark` (MIT): +60 KB gzip on top of the text module in WebAssembly.
- **The JSON document format** is the flow model with `serde` derives (the `serde` and `schema` features), so it needs no extra crate.
- **WebAssembly:** all three go in the text module, which already carries the builder and text. ADR 0016 §6's text-module budget rises from 1,200,000 to 1,280,000 B gzip (§9), an amendment the owner approves with this ADR.

### 6. Inputs

**Markdown:** CommonMark with four extensions: tables, footnotes, strikethrough and task lists. Front matter is plain `key: value` lines (`title`, `lang`, `author`, `subject`), so no YAML parser is needed. Raw HTML is dropped with the warning `markdown-html-ignored`. An image the caller didn't supply, or a remote one, is dropped with `markdown-image-missing`: nothing is fetched. Maths isn't parsed, so `$…$` stays text. Warnings locate the problem by byte offset, never by quoting the text ([ADR 0026 §4](0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean)).

**JSON documents:** `laminapdf.document/1`, the flow model as JSON (camelCase names, block kinds tagged by `type`), with its JSON Schema committed and versioned under [ADR 0031 §6](0031-cli-and-mcp-contract.md#6-json-output)'s rule. Fonts and images are named in the document and supplied beside it as bytes.

**Templates** are placeholders inside the document JSON: `{ "$data": "/pointer" }` for a value, `{ "$each": "/array", "do": … }` to repeat a block or table row, and `{ "$if": "/pointer", "do": … }`. Pointers are JSON Pointers (RFC 6901). There are no expressions and no code, and a filled template is still a valid document. A text template engine was measured and rejected: MiniJinja adds 125 KB gzip, and text-level templating can write broken JSON that only fails after filling.

**For agents and the declarative layer:** the docs and the MCP tool offer Markdown for prose and `laminapdf.document/1` for data, beside each other. The document tree answers pdf-library §9 question 3: it is the declarative layer, and a JSX layer, if one comes, maps onto it.

### 7. Surfaces

- **CLI:** `laminapdf build <input.md | doc.json | -> -o OUT`, with `--font ROLE=FILE`, `--data FILE` for templates, `--page-size`, `--profile`, `--lang` and `--title`. `--json` prints `laminapdf.build/1` (output, page count, bytes, tagged, profiles, warnings). This amends [ADR 0031 §2](0031-cli-and-mcp-contract.md#2-grammar), whose `render` already means rasterising pages, and §6.
- **MCP:** `laminapdf_build`, listed only with `--allow-write`, takes Markdown or a document and its data inline, or a path, and writes inside the roots. This amends [ADR 0031 §9](0031-cli-and-mcp-contract.md#9-mcp-defaults-for-l4c) and answers trust and proof §9 question 2.
- **TypeScript:** `builder.markdown(text, options)` and `builder.flow(document, { data })`, with the document as typed objects generated from the committed schema, fonts as `Font`s and images as `Image`s by name.
- **Jobs:** `Builder::flow` runs in one call at first. The spike's report took about 310 ms per pass in Node 24 (619 ms for two). If T4b's 100-page benchmark exceeds §9's budget in WebAssembly, a `FlowJob` stepped by page follows [ADR 0028 §3](0028-facade-rules-jobs-and-cancellation.md#3-jobs).

### 8. Fonts

The front ends use the fonts the caller gives, by role (body, bold, italic, bold italic, mono, headings). Without fonts they use the standard 14 Helvetica family and Courier, which set Latin text only, unshaped. PDF/UA-1 refuses those fonts (`font-not-embedded`), and the CLI's hint for that error names `--font`. No font is bundled: a Noto Sans subset would cost more than the flow layer itself. pdf-library §9 question 2 (an optional fonts package) stays open.

### 9. Budgets

| What | Budget | The spike measured |
| --- | --- | --- |
| Two-page invoice, native | ≤ 10 ms | 5.6 ms (two passes) |
| Two-page invoice, Node 24, best of 30 | ≤ 25 ms | 10.7 ms (two passes) |
| 100-page report, native | ≤ 250 ms | about 190 ms per pass |
| 100-page report, Node 24, best of 10 | ≤ 500 ms ([pdf-library §4](../plans/pdf-library.md#4-requirements)) | about 310 ms per pass |
| Text module, gzip | 1,400,000 B (amended 2026-11-11; 1,360,000 from 2026-11-10; first 1,280,000, from 1,200,000) | baseline 819 KB in the probe; paginator +19 KB, Markdown +60 KB; measured with T4i: 1,316,930 B, the flow layer about +60 KB and Markdown about +90 KB; with T4j: 1,391,637 B, JSON documents and templates about +75 KB |

Times are recorded, not asserted ([ADR 0018](0018-test-strategy.md)): T4b's benchmark joins the nightly, and the invoice joins `cargo xtask pdf-runtimes`. The text module's budget covers the flow layer, tables, notes, Markdown and templates together. `cargo xtask size` enforces it.

### 10. What lands when

| Todo | What |
| --- | --- |
| [T4f](../todos/done/T4f-flow-builder-groundwork.md) | The builder groundwork: measuring text without drawing it, elements that continue on later pages, artifact subtypes, Unicode for marks under PDF/UA-1 |
| [T4g](../todos/done/T4g-flow-styled-spans.md) | Styled spans in one paragraph: fonts, sizes, rises and languages mixed on a line |
| [T4b](../todos/done/T4b-flow-layout-core.md) | The flow layer over T4f and T4g: paragraphs of styled spans, pagination, keep rules, page templates and fields, "Page X of Y", tagging |
| [T4c](../todos/done/T4c-flow-tables-notes-contents.md) | Tables across pages |
| [T4h](../todos/done/T4h-flow-notes-contents-outline.md) | Footnotes, a contents page and the outline |
| [T4d](../todos/done/T4d-markdown-documents.md) | Markdown in Rust, with an example |
| [T4i](../todos/done/T4i-build-command-and-bindings.md) | `laminapdf build`, `laminapdf_build` and the TypeScript `markdown` call |
| [T4e](../todos/done/T4e-json-documents-and-templates.md) | `laminapdf.document/1` and templates in Rust |
| [T4k](../todos/ready/T4k-json-documents-in-the-cli-and-mcp.md) | the same from `laminapdf build` and its MCP tool |
| [T4j](../todos/done/T4j-documents-in-typescript.md) | Typed documents and `builder.flow` in TypeScript |

## Consequences

- Lamina PDF owns a layout engine, with the work layout engines spend years on: keep rules, tables across pages, notes and contents pages. The todos above bound it, and §3's list names what stays out.
- Generated pages live in the document's overlay, so "generate, then edit" and appending flowing pages to an open document keep working, with undo.
- The core keeps its rules: no threads, no platform code, bytes in and bytes out, one shaper (HarfRust) and one font stack.
- Typst's 0.x releases don't move our API. The cost is features Typst has and we lack: maths, bibliographies, a scripting language.
- The text module grows by up to 150 KB gzip, the cost of flowing layout and both inputs in every runtime that loads text.

## Alternatives considered

- **Typst embedded** (`typst` and `typst-pdf`): rejected for its size (+7.36 MB gzip), its 147 crates with thread pools, its API churn, its finished-bytes output and its silent missing glyphs (§2).
- **Typst's layout drawn through `Surface`** (the hybrid): rejected. It needs a glyph-run API and a tagger of our own, and keeps Typst's size (§2).
- **krilla used directly:** krilla draws fixed positions and has no flowing layout, so it needs an engine above it, and it only returns finished bytes ([ADR 0005 §6](0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges)).
- **Taffy with our own pagination:** Taffy (MIT) lays out flexbox and grid boxes on one infinite surface; text measurement, line breaking, pagination and tagging would still be ours. It adds a dependency for the part of the work that is easiest ([research 05 §8.3](../research/pdf/05-creating-pdfs-from-code.md#83-build-versus-wrap-which-existing-work-to-stand-on)).
- **HTML and CSS through a browser:** a headless browser can't run in the core, a phone or a Worker, and its PDFs are untagged unless asked. It belongs to [E13f](../todos/ready/E13f-office-and-html-to-pdf.md) as an opt-in server job.
- **Optimal (Knuth–Plass) page breaking:** better page breaks for books, at the cost of layout passes and a harder model. Greedy breaking with keep rules covers the requests the research found.
- **A text template engine** (MiniJinja, Typst's own language): +125 KB gzip for MiniJinja, and text-level templates can write invalid documents (§6).
