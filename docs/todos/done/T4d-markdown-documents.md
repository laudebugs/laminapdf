# T4d — Markdown in, flowing pages out, in Rust

| Field | Value |
| --- | --- |
| ID | `T4d` |
| Status | `done` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4b](./T4b-flow-layout-core.md), [T4c](./T4c-flow-tables-notes-contents.md), [T4g](./T4g-flow-styled-spans.md), [T4h](./T4h-flow-notes-contents-outline.md) |
| ADRs | Implements [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md): the Markdown dialect ([§6](../../decisions/0032-flowing-layout-and-document-inputs.md#6-inputs)), the front end as a `markdown` feature of `laminapdf-build` ([§5](../../decisions/0032-flowing-layout-and-document-inputs.md#5-placement)), the default fonts ([§8](../../decisions/0032-flowing-layout-and-document-inputs.md#8-fonts)) and the tag mapping ([§4](../../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging)). Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md#rust-cargo-deny) (the parser's licence) and [0026 §4](../../decisions/0026-error-and-diagnostic-contract.md#4-no-document-text-in-message-hint-or-didyoumean) (no document text in messages) |
| Branch | `feat/t4d-markdown-in` |
| PR title | `feat(build): Markdown in, flowing pages out` |
| Size | L (≈ 600–750 lines: the front end, three new blocks, the façade feature, the example and the fuzz target; the fixture and goldens don't count) |
| PR | [#354](https://github.com/laudebugs/laminapdf/pull/354) |

## Goal

Turn Markdown into flowing, tagged pages from Rust: `Builder::markdown(text, &MarkdownOptions)` maps CommonMark and four extensions onto the flow layer and lays it out. Writing in Markdown or another plain markup is the largest generation request on Hacker News, about 1,140 comments, and people say what they would do with it: "if you have a markdown->PDF function included, where I can send in markdown and get PDF, that would solve quite many needs" ([46325132](https://news.ycombinator.com/item?id=46325132)). Today they chain tools for it ("MD-->(Pandoc-->(typst))--> PDF", [43232394](https://news.ycombinator.com/item?id=43232394)). Talk of AI near generation rose from 6 to 263 comments, and Markdown is the format agents already write. The CLI command, the MCP tool and the TypeScript call come next, in [T4i](../ready/T4i-build-command-and-bindings.md).

## Context

- [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md) decides the route: Lamina PDF's own flow layer in `laminapdf-build` ([§2](../../decisions/0032-flowing-layout-and-document-inputs.md#2-our-own-engine-over-surface-with-typst-as-the-measuring-stick)), with Markdown as one of its two inputs. [§6](../../decisions/0032-flowing-layout-and-document-inputs.md#6-inputs) fixes the dialect: CommonMark with tables, footnotes, strikethrough and task lists; front matter as plain `key: value` lines; raw HTML dropped with `markdown-html-ignored`; images the caller didn't supply, or remote ones, dropped with `markdown-image-missing`; maths left as text; warnings located by byte offset. [§10](../../decisions/0032-flowing-layout-and-document-inputs.md#10-what-lands-when) splits the work: this todo is Markdown in Rust, and T4i brings it to the CLI, MCP and TypeScript.
- The flow layer this todo feeds: [T4b](../done/T4b-flow-layout-core.md) (blocks, pagination, keep rules, page templates, fields, tagging), [T4g](../done/T4g-flow-styled-spans.md) (styled spans, links and languages inside one paragraph), [T4c](../done/T4c-flow-tables-notes-contents.md) (tables across pages) and [T4h](../done/T4h-flow-notes-contents-outline.md) (footnotes, a contents page, the outline and a named destination per heading).
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.1 (1,424 matches, 8 of 10 on topic), §5 (AI near generation) and §6 point 8; the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- The parser: [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark) (MIT), a CommonMark parser with options for tables, footnotes, strikethrough, task lists and metadata blocks, and no `std::fs`. ADR 0032 §5 measured it at 78 KB gzip on its own in WebAssembly. Its licence and its dependencies go through `cargo xtask deny`.
- Examples: the [examples guideline](../../guidelines/examples.md). The Rust example appears on the "Pages that flow" guide (T4b's) with a Rust tab; T4i adds the TypeScript twin, its pair in `tools/xtask/docs-snippets.toml` and the TypeScript tab.
- `TagKind` has `Code` but no `BlockQuote` (PDF 1.7's block quote, ISO 32000-1 §14.8.4).
- No message or hint carries document text: `crates/laminapdf/tests/error_contract.rs` checks the façade's against a marker string, and the same rule covers Markdown. A warning about raw HTML gives its position, never the HTML.

## Scope

**In scope**

- **The Markdown front end,** a `markdown` feature of `laminapdf-build` in `src/markdown/`, over `pulldown-cmark` (ADR 0032 §5). CommonMark with tables, footnotes, strikethrough and task lists, and front matter with `title`, `lang`, `author` and `subject` as plain `key: value` lines, so no YAML parser is needed (§6).
- **The mapping,** one construct to one flow block and structure type:

  | Markdown | Flow (T4b, T4g, T4c, T4h, this todo) | Tag |
  | --- | --- | --- |
  | `#` to `######` | `Heading` | `H1`–`H6` (a skipped level stays an error under PDF/UA-1: `heading-level-skipped`) |
  | paragraph, hard break | `Paragraph`, line break | `P` |
  | `*em*`, `**strong**`, `` `code` ``, `~~strike~~` | spans in the italic, bold and mono fonts; a span struck through | `Span`, `Code` |
  | link | a link span: a URL, or `#slug` to that heading's destination | `Link` |
  | image | `Figure`, its text as alt text, its bytes from the caller | `Figure` with `/Alt` |
  | list, task list | `List` (an ordered list's start number; ☐ and ☑ markers) | `L`, `LI`, `Lbl`, `LBody` |
  | block quote | `Quote` (new block) | `BlockQuote` (new `TagKind`) |
  | code block | `CodeBlock` (new block: mono, wrapped at the body's width) | `P` holding `Code` |
  | thematic break | `Rule` (new block) | `Layout` artifact |
  | table | `Table`, column alignment from the delimiter row, the first row as header | `Table`, `THead`, `TBody`, `TR`, `TH`, `TD` |
  | footnote | a note | `Reference`, `Note` |
  | front matter | metadata and language | `/Title`, `/Lang` |

- **What is dropped,** as warnings in T4b's `buildWarnings` group, each with its `docs/errors/` page and a location (a byte offset into the Markdown, in its `data`) instead of the text: `markdown-html-ignored` (raw HTML) and `markdown-image-missing` (an image the caller didn't supply, or a remote URL: nothing is fetched).
- **Options** (`MarkdownOptions`, new): fonts by role (body, bold, italic, bold italic, mono, headings), sizes, the page template (by default A4, its margins, and a "Page {page} of {pages}" footer), and images as a map from the link target to an `Image`. Without fonts, the front end uses the standard 14 Helvetica family and Courier (ADR 0032 §8). They set Latin text only, unshaped, and PDF/UA-1 refuses them (`font-not-embedded`). No font is bundled.
- **The façade:** a `markdown` feature with `laminapdf::markdown` and `Builder::markdown(text, &MarkdownOptions)` (new), returning what the flow added and its warnings.
- **The example:** `crates/laminapdf/examples/markdown_to_pdf.rs` over `fixtures/markdown/report.md` (new: every construct in the table, an Arabic and a Devanagari paragraph, a table across pages, footnotes, a contents page and an image), with byte and render goldens and veraPDF under PDF/UA-1. It takes its fonts and image as parameters, as the other examples do.
- **A `markdown` fuzz target:** any text in, a document or a coded error out, never a panic.

**Out of scope** (and which todo covers it)

- `laminapdf build` for Markdown, its `laminapdf.build/1` report, the MCP tool `laminapdf_build`, `WasmBuilder.markdown`, `builder.markdown` in `@laminapdf/sdk`, the TypeScript example and the WebAssembly size check: [T4i](../ready/T4i-build-command-and-bindings.md).
- JSON documents and templates: [T4e](../ready/T4e-json-documents-and-templates.md).
- Maths (`$…$` stays text), raw HTML, syntax highlighting, styling beyond `MarkdownOptions` (no CSS), remote images, and Markdown through Pandoc or Typst (ADR 0032 §2 and §6).
- Fonts bundled with the library: [pdf-library §9](../../plans/pdf-library.md#9-open-questions) question 2, which ADR 0032 §8 leaves open.

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`, and `/writing` for the docs pages
- [x] Read ADR 0032 and the flow modules from T4b, T4g, T4c and T4h
- [x] Add `pulldown-cmark` to `[workspace.dependencies]` (default features off, as the crate needs), behind the `markdown` feature, and run `cargo xtask deny`
- [x] The new blocks `Quote`, `CodeBlock` and `Rule` in the flow model, with their layout, pagination and tags; `TagKind::BlockQuote` with `laminapdf-wasm`'s name table and `cargo xtask gen boundary`
- [x] `src/markdown/`: front matter, the event-to-block mapping in the table, slugs for `#` links, `MarkdownOptions` with its defaults, the two warnings with their `codes.json` entries and `docs/errors/` pages, then `cargo xtask gen codes`
- [x] The façade's `markdown` feature and `Builder::markdown`
- [x] The fixture `fixtures/markdown/report.md` with its provenance row in `fixtures/README.md`, the Rust example and its goldens (`cargo xtask golden --update`, `cargo xtask render-golden --update`)
- [x] The fuzz target: `fuzz/fuzz_targets/markdown.rs`, its `[[bin]]` in `fuzz/Cargo.toml`, `FUZZ_TARGETS` in `tools/xtask/src/harness.rs`, seeds from the fixture
- [x] The docs (Docs to update)

## Acceptance criteria

- [x] Each row of the mapping table has a test in `crates/laminapdf-build/tests/markdown.rs` (new) that builds the construct and reads back its block and structure type (`laminapdf_model::Document::structure()`)
- [x] `markdown_to_pdf` builds the golden's bytes from `fixtures/markdown/report.md`, passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`, and extracts its Arabic and Devanagari paragraphs as written
- [x] Without fonts, a Latin document builds with the standard 14 fonts, and the same document under PDF/UA-1 fails with `font-not-embedded`
- [x] Raw HTML gives `markdown-html-ignored`, and a missing or remote image gives `markdown-image-missing`, each with a byte offset in its `data`
- [x] Markdown whose raw HTML and image targets hold a marker string gives warnings whose messages and hints lack it
- [x] `pulldown-cmark` and its dependencies pass `cargo xtask deny`
- [x] The `markdown` fuzz target runs in the full lane (`cargo xtask fuzz --all`) without a crash
- [x] `cargo xtask check docs` passes with the Rust example shown on the "Pages that flow" guide

## Post-merge checks

- [ ] The nightly fuzz job runs the `markdown` target for its 10 minutes without a crash

## Tests

- `crates/laminapdf-build/tests/markdown.rs` (new): the mapping table, front matter, slugs, the two warnings, the defaults without fonts.
- `crates/laminapdf/tests/examples.rs` for the Rust example.
- `crates/laminapdf/tests/error_contract.rs`: the marker check over Markdown warnings.
- The `markdown` fuzz target.

## Docs to update

- [x] The "Pages that flow" guide: a section on Markdown with the Rust example in a Rust tab (T4i adds the TypeScript tab and the CLI command)
- [x] `CLAUDE.md`: the Building API paragraph and the fuzz target list
- [x] `fixtures/README.md`: the Markdown fixture and the new goldens
- [x] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Size.** If the PR passes about 800 lines, move the three new blocks into a PR of their own before the front end.
- **Fonts.** Without fonts, only Latin text sets, and PDF/UA-1 fails. The rustdoc of `MarkdownOptions` must say so plainly; a bundled default font stays the owner's call (pdf-library §9 question 2).
- **Markdown dialects differ.** Users will bring GitHub's and Pandoc's extras. ADR 0032 §6's list is the contract, and anything else is plain text or a coded warning, never a silent guess.
- **Untrusted Markdown from agents.** Images come only from the caller's map, and nothing is fetched. The fuzz target checks that odd input ends in a document or a coded error.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Standard 14 fonts in flows,** a prerequisite of the default fonts: `HarfRust::shape` lays a standard 14 font out one glyph per character with its AFM width, `itemize` reads its coverage from the font's own table, and the builder's writer draws such runs a byte per character through `FontRegistry::encode`. `pdf_run` still refuses them (`font-not-shapable`). Shaping and drawing both skip what `Standard14::draws_nothing` (new) names: controls, U+2028 and U+2029 (a flow's hard break) and zero-width format characters. No golden changed.
- **The flow model:** `Inline::Code` (`Paragraph::code`, tagged `Code` inside the paragraph), `TextStyle::with_strikethrough` (the line drawn after the text as a `Layout` artifact, outside the element's marked content), `List::with_start`, `ListItem::with_marker`, and the blocks `Quote(Vec<FlowBlock>)` (`TagKind::BlockQuote`, new, indented 18 pt), `CodeBlock(String, TextStyle)` (a `P` holding `Code`, indented 12 pt, wrapped at the body's width) and `Rule` (a `Layout` artifact). T4h's `NoteRule` placement became the shared `Rule`. Contents pages list headings inside quotes.
- **The front end** (`src/markdown/`): up to three passes over pulldown-cmark's events: the first finds the headings the flow will number and their slugs (GitHub's rule, `-1`, `-2` for repeats), the second (only when there are footnotes) builds the footnote bodies with their `#slug` links, the third the blocks, so a note or a link may come before its target. A footnote reference inside a footnote stays `[^label]`, since a note's body holds no notes. A tight list item's text ends where a nested list, quote, code block or table starts. An image or inline code inside a heading, a cell, an item or a note becomes text there; inline code inside a link is linked but not tagged `Code`. Front matter is applied after the flow lays out, merged into the metadata the caller set. A `#slug` without a heading is plain text with no link; a footnote reference without a definition stays as `[^label]`. Task markers are ☐ and ☑, or `[ ]` and `[x]` when the body font lacks them. Headings use the bold font at 2, 1.6, 1.3 and 1.15 times the body size, and levels 5 and 6 at the body size. Inline HTML gives one warning per tag; an HTML block one warning.
- **Deviation: `MarkdownBuildOptions`.** The options are `MarkdownBuildOptions`, since `laminapdf-extract` already has a `MarkdownOptions` (how extraction writes Markdown) and the maintainability check allows one public type per name.
- **Options:** `MarkdownFonts` has regular, bold, italic, bold italic, mono and fallbacks; headings take the bold font rather than a role of their own. `MarkdownBuildOptions` adds `with_contents(levels)` for a contents page at the start.
- **Warnings:** `markdown-html-ignored` (`unsupported`) and `markdown-image-missing` (`usage`), each with `offset` in its data and no location (the offset is in the Markdown, not the PDF). The façade maps `offset` in `data.rs`. `tests/error_contract.rs`'s marker diagnostics include a Markdown build whose HTML, alt text and image target hold the marker.
- **Deviation: the façade's feature is on by default.** `markdown` joins `text` and `saslprep` in the façade's defaults, so `tests/examples.rs` and `cargo xtask examples` build `markdown_to_pdf` without a feature flag. The workspace takes `laminapdf` without defaults, so the WASM modules and the CLI turn it on only where T4i needs it.
- **The example** sets A5 pages so the 12-row table crosses a page break, and uses Noto Sans for code (no monospaced fixture font: Courier would fail PDF/UA-1). It fails on any Markdown warning. Its image is `images/photo-rgb.jpg`, with alt text that describes that test picture. Its second page renders at SSIM 0.965 against its render golden through `laminapdf-render`'s hayro adapter, so `laminapdf-cli`'s `tests/render.rs` gives it a bar of 0.95, as it does the other flowing examples.
- **The fuzz target** shares its entry point with `laminapdf-build`'s `tests/regressions.rs` (`tests/common/fuzz_entry.rs`), which replays `fixtures/markdown/` and `fuzz/seeds/markdown/`; the report fixture fails with `missing-glyph` in both of the target's fonts, so the Latin seed carries every construct through to a saved document.
