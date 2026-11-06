# T4b — Flowing layout: paragraphs across pages, keep-together, running headers and footers, page X of Y

| Field | Value |
| --- | --- |
| ID | `T4b` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4a](./T4a-adr-0032-layout-engine.md) |
| ADRs | Implements ADR 0032 (written by [T4a](./T4a-adr-0032-layout-engine.md)); follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (the builder writes into the overlay), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space; [§5](../../decisions/0027-coordinates-units-and-page-indexing.md#5-text-placement): a run's `y` is its baseline), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (codes) and [0018](../../decisions/0018-test-strategy.md) (goldens) |
| Branch | `feat/t4b-flow-layout-core` |
| PR title | `feat(build): flowing layout with keep rules, running headers and footers, and page X of Y` |
| Size | L (≈ 1,100–1,400 lines; split at the line in Risks if it grows) |
| PR | — |

> **Written to the expected decision.** This todo assumes ADR 0032 chooses an engine of our own over `Surface`, as the research report recommends. T4a may reshape it: if the ADR decides otherwise, T4a rewrites this file before anyone claims it.

## Goal

Give `laminapdf-build` a flow layer. The caller hands it blocks of text and a page template, and it makes as many pages as the content needs. A paragraph that doesn't fit continues on the next page, a heading stays with the block after it, a split paragraph leaves at least two lines at the foot and at the head of a page, and running headers and footers repeat on every page, with footers that can say "Page 3 of 12". On Hacker News, page control is the generation request with the clearest gap: about 345 comments ask for page breaks, orphans and widows, running headers and page numbers, and a 2017 commenter called sensible page breaks "the holy grail of PDF generators" ([15071383](https://news.ycombinator.com/item?id=15071383)). This todo is Rust only. Tables, footnotes and contents pages come in T4c, and the WASM and TypeScript surfaces in T4d and T4e.

## Context

- [T4a](./T4a-adr-0032-layout-engine.md) and ADR 0032: the decision, the spike's prototype (experiment A) and its numbers.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.6 (control over pages, about 345 comments) and §2.4 (pagination from HTML, about 100: "the biggest gripe with generating (long) PDFs from HTML are things such as page numbering, orphans and widows, semantically correct word-wrapping, page margins, etc...", [16959758](https://news.ycombinator.com/item?id=16959758)); the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- The builder: `Surface::text_box` (`crates/laminapdf-build/src/surface/mod.rs`; `TextBoxOptions`, `TextLayout`, `LineBox` and `Overflow` in `surface/textbox.rs`) lays one paragraph in one font and size into one rectangle. The crate-private `lay_out` and `Spec` in `surface/text.rs` shape a paragraph once. `Builder::page` hands out one `PageBuilder` at a time.
- `laminapdf-text`'s `break_lines` (`src/layout.rs`, 702 lines against the 800-line module budget) breaks at one size, and its font list is a fallback list, not a set of styles: a bold word inside a paragraph isn't possible yet. `Line` and `LineRun` already hold runs in several fonts per line.
- Tagging: `src/tag/elem.rs` collects elements page by page and gives them object numbers when their page is written, and `Surface::tag` documents "Elements are per page: one doesn't continue on the next page". `src/tag/artifact.rs` writes `/Artifact << /Type /Pagination >>` without a `/Subtype`. `src/profile/checks.rs` checks heading levels under PDF/UA-1 (`heading-level-skipped`).
- ISO 32000-2 §14.7.5.2 (a marked-content reference, an `/MCR` dictionary with its own `/Pg`, for an element's content on another page) and §14.8.2.2 (artifacts: `/Type /Pagination` with `/Subtype /Header` or `/Footer`); ISO 32000-1 §14.8.4 (the standard structure types the builder writes); the Matterhorn Protocol's checkpoints 14 (headings), 16 (lists) and 18 (page headers and footers). Research 03 [§9.5](../../research/pdf/03-text-and-fonts.md#95-generating-tagged-pdf-from-code) (generating tagged PDF) and [§7.6](../../research/pdf/03-text-and-fonts.md#76-justification) (justification, which stays out).
- [E6d](./E6d-watermarks-headers-footers.md) stamps headers, footers and page numbers onto existing documents. This todo defines the field syntax (`{page}`, `{pages}`) once, in `laminapdf-build`, for E6d to reuse.
- `laminapdf-extract` drops `Artifact`-marked runs and running headers and footers ([L3d](../done/L3d-extract-structure-and-furniture.md)), which gives a round-trip check of the tags.
- [B5d](../done/B5d-text-report-generation-performance.md): the `report` bench (`crates/laminapdf/benches/report.rs`, 39 ms natively for 100 pages); [T1c](./T1c-publish-performance-numbers.md) publishes generation timings.

## Scope

**In scope**

- **The flow model** (`crates/laminapdf-build/src/flow/`, new; names settle in review). A `Flow` holds `Block`s and a `PageTemplate`. The blocks:
  - `Paragraph`, made of inline spans: text in a `TextStyle` (fonts, size, colour, a baseline rise for superscripts, a language), links (`LinkTarget`) and line breaks.
  - `Heading`, levels 1 to 6.
  - `List`: bulleted or numbered, nested, with a hanging indent.
  - `Figure`: an `Image` with its alt text, at a given size or the frame's width.
  - `Spacer`, `PageBreak`, and `Keep` (a group of blocks kept on one page).
  - A `ParagraphStyle` gives alignment, line height, space before and after, `keep_with_next`, `widows`, `orphans` (2 and 2 by default) and `break_before`.
- **Styled spans in one paragraph** (`laminapdf-text`): line breaking over spans in different fonts and sizes. Each item is shaped at its span's size, and each line takes its ascent and descent from its tallest span. A single span gives exactly the lines `break_lines` gives.
- **Pagination:** greedy and deterministic. A paragraph splits between lines and honours widows and orphans. A heading keeps with the next block; a `Keep` group and a figure move whole; a `PageBreak` starts a new page. When the next page's body is a different width (a first-page template), the rest of the paragraph is broken again at that width.
- **Page templates:** page size, margins, a header and a footer, each with start, centre and end slots of text and fields:
  - `{page}`: the 1-based page number in the document;
  - `{pages}`: the document's page count once the flow is added (pages added later aren't counted);
  - `{section}`: the last heading of a chosen level that started on or before the page;
  - `{title}`: the document title.

  A first-page variant can differ, for example a cover without a header.
- **Page X of Y:** the flow lays out every page before drawing any, so `{pages}` is known when the footers are drawn. No deferred objects.
- **Tagged output,** by default unless ADR 0032 decides otherwise: `P`, `H1`–`H6`, `L`, `LI`, `Lbl`, `LBody`, `Figure` with `/Alt`, `Link`, and `Span` with `/Lang` for a span in another language. A paragraph or list split across pages stays one element, whose `/K` points at its content on later pages through MCR dictionaries. Headers and footers are `Pagination` artifacts with `/Subtype /Header` or `/Footer`. A flow built under the PDF/UA-1 profile passes `Builder::finish`'s checks and veraPDF.
- **Codes,** each with its `docs/errors/<code>.md` page:
  - `flow-content-too-large` (`buildErrors`): a line or a figure taller than the page's body.
  - `flow-keep-broken` (a warning, in a new `buildWarnings` group): a `Keep` group or a keep-with-next chain taller than a page, split anyway.
- **The façade:** `laminapdf::flow` (re-exports), `laminapdf::Builder::flow`, and the flow's warnings in `Document::warnings()`.
- **An example:** `crates/laminapdf/examples/flowing_report.rs`, a 12-page report with a cover, headings, lists, a figure, an Arabic and a Devanagari paragraph, a running header with the section title and a "Page X of Y" footer, with byte and render goldens.
- **A benchmark:** a 100-page flowing report in the nightly benches.

**Out of scope** (and which todo covers it)

- Tables, footnotes, contents pages, and the outline and named destinations made from headings: [T4c](./T4c-flow-tables-notes-contents.md).
- Block quotes, code blocks, rules, and Markdown: [T4d](./T4d-markdown-and-json-templates.md). The JSON form of a flow and the TypeScript API: [T4e](./T4e-json-documents-and-templates.md). Until T4d the flow is Rust only, and link-time optimisation leaves it out of the WASM modules, so their sizes don't change.
- Justification, hyphenation, columns, floats, and odd and even templates with mirrored margins: later, as ADR 0032 records.
- Flowing pages appended to an open document (`Builder::on`): appended pages are never tagged today.
- A stepped job for long flows ([ADR 0028 §3](../../decisions/0028-facade-rules-jobs-and-cancellation.md#3-jobs)): only if ADR 0032 asks for one.

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [ ] Read ADR 0032 and the spike's experiment A, and check this todo against the decision
- [ ] `laminapdf-text`: split `src/layout.rs` into a `src/layout/` folder (the paragraph breaker and the line types in separate modules) so neither passes the 800-line budget. Then add styled spans: a span type and a breaking function over spans (both new; for example `Span { range, fonts, size, rise }` and `break_spans`). Items split at span boundaries, each item is shaped at its span's size, a line's metrics come from its tallest span, and each `LineRun` records its span. Re-export the new items from the crate root, as every item there is
- [ ] `flow/model.rs`: the types in Scope, with builder-style constructors and their defaults; no serde yet (T4e adds it)
- [ ] `flow/measure.rs`: each block laid out at a frame width through `surface::text::lay_out` or the span breaker, giving line heights. Keep each line's shaping, so a line is shaped once and drawn from the same result
- [ ] `flow/paginate.rs`: the paginator over measured blocks. It produces pages of placed lines and figures, applies the keep rules with bounded backtracking (a chain moves at most once), and raises `flow-keep-broken` and `flow-content-too-large`
- [ ] `flow/template.rs`: regions, slots, fields and the first-page variant; `{section}` tracked while paginating; headers and footers laid out once the page count is known
- [ ] `flow/emit.rs`: one `Builder::page` per laid-out page. Placed lines are drawn through a crate-private `Surface` command that takes measured lines (no second shaping), figures through `Surface::figure`, links per line fragment, and the tags
- [ ] `src/tag/`: let an element continue on the next page. Its object is written when it closes, its content on later pages goes into `/K` as MCR dictionaries with `/Pg`, and each page's `ParentTree` entries point at it. Keep it crate-private: the flow needs it, and a public API can follow. In `artifact.rs`, give `Pagination` artifacts a `/Subtype` (check whether `pdf-writer`'s artifact writer has a setter)
- [ ] `Builder::flow(&Flow) -> Result<FlowReport>` (new): the pages it added and its warnings
- [ ] Codes: `flow-content-too-large` in `buildErrors`, and a new `buildWarnings` group in `fixtures/codes.json` with `flow-keep-broken`, its row in `GROUPS` (`tools/xtask/src/codegen.rs`) and both `docs/errors/<code>.md` pages; then `cargo xtask gen codes`. A `laminapdf_build::Warning` type (new) with `code()`, `kind()` and `location()`, like `laminapdf_text::Warning`; add `laminapdf-build` to the `Warning` entry of `tools/xtask/maintainability-duplicates.toml`, whose reason ("one warning type per crate that warns") covers it
- [ ] The façade: `pub mod flow`; `Builder::flow`; a `Warning::Build` variant (new), so `Document::warnings()` lists the flow's warnings beside the fonts'
- [ ] The example: its byte golden through `crates/laminapdf/tests/examples.rs` (`cargo xtask golden --update`), its render goldens (`cargo xtask render-golden --update`) and its facts (`tools/fixtures/build_facts.py`)
- [ ] The bench: a 100-page flowing report beside `report` in `crates/laminapdf/benches/` (a `[[bench]]` entry in `crates/laminapdf/Cargo.toml`), which `cargo xtask bench` runs
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] A paragraph that doesn't fit the rest of a page continues on the next, and the text `laminapdf-text`'s `TextExtractor` reads from the two pages equals the input
- [ ] Widows and orphans: with room for one line at the foot of a page, a 10-line paragraph starts on the next page; with room for nine lines, it splits eight and two
- [ ] A heading with room for one more line moves to the next page with the paragraph after it
- [ ] A `Keep` group taller than a page is split with `flow-keep-broken`, and a figure taller than the body fails with `flow-content-too-large`
- [ ] In the 12-page example, every page after the cover shows the running header with its section's title and the footer "Page n of 12", read back by extraction; the cover has neither
- [ ] A paragraph split across two pages is one `P` element with content on both pages (`laminapdf_model::Document::structure()`); headers and footers are `Pagination` artifacts with their `/Subtype`; `laminapdf-extract` lists them in `dropped`, and none of their text is in a block
- [ ] The example builds under PDF/UA-1, passes `cargo xtask verapdf` and `qpdf --check` (`cargo xtask examples`), and matches its byte golden on two runs in a row
- [ ] The Arabic and Devanagari paragraphs, each split across a page break, extract as their input with pdf.js and pdftotext (`fixtures/build/facts.json`)
- [ ] One span gives the same lines as `break_lines` for the inputs in `fixtures/shaping/`; a line with two sizes takes its height from the larger
- [ ] The 100-page flowing report runs within the budget ADR 0032 sets (proposal: at most twice the `report` bench's time on the same machine)
- [ ] `cargo xtask gen codes --check` and `cargo xtask check maintainability` pass

## Post-merge checks

- [ ] The nightly `bench` job records the flowing report in `history.jsonl` (`cargo xtask bench --history`), so T1c can publish it

## Tests

- Unit tests in `flow/paginate.rs` for each keep rule, widows and orphans, page breaks, a first page of another width and both codes. A proptest: random flows place every line exactly once, inside the body, in order, and give the same pages twice.
- Unit tests in `laminapdf-text`'s layout modules for spans: one span, mixed sizes, a bold Latin word inside an Arabic paragraph.
- `crates/laminapdf-build/tests/flow.rs` (new): structure across pages, artifact subtypes and the fields. `crates/laminapdf/tests/flow.rs` (new): the round trip through `Document::extract`, since the façade has both the builder and extraction.
- Goldens: `fixtures/golden/examples/flowing_report.pdf`, its render goldens in `fixtures/render-golden/`, and its facts in `fixtures/build/facts.json`.
- The `examples` and `verapdf` runs over the new example; the bench.

## Docs to update

- [ ] A "Pages that flow" guide on the docs site (`apps/docs/src/content/docs/guides/flowing-pages.mdx`, new) that shows the Rust example; T4e adds the TypeScript tab
- [ ] `laminapdf-build`'s README and `lib.rs` docs; `laminapdf-text`'s README for spans
- [ ] `CLAUDE.md`'s Building API and Fonts API paragraphs
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Size.** If the PR passes about 1,400 lines, split it here: styled spans in `laminapdf-text`, with the crate-private command that draws a measured line in several fonts, go first as their own PR, and the flow follows.
- **Elements across pages change an invariant of `src/tag/`:** today an element is numbered when its page is written. Check the `ParentTree` against veraPDF early.
- **Keep chains can fight widows and orphans.** Bound the backtracking so that every flow ends, and prefer breaking a keep (with the warning) to searching longer.
- **Line metrics with fallback fonts.** An Arabic run inside a Latin paragraph can make lines uneven. Take the tallest span's metrics, and test the case.
- **ADR 0032 may choose another route,** and T4a then rewrites this todo.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
