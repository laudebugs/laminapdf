# T4c — Flowing layout: tables across pages with repeated headers and kept totals

| Field | Value |
| --- | --- |
| ID | `T4c` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4b](./T4b-flow-layout-core.md) |
| ADRs | Implements [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md) [§3](../../decisions/0032-flowing-layout-and-document-inputs.md#3-the-layout-model) (tables) and [§4](../../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging) (one `Table` element across pages); follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space, 1-based page numbers in what users read) and [0026](../../decisions/0026-error-and-diagnostic-contract.md) (codes) |
| Branch | `feat/t4c-flow-tables` |
| PR title | `feat(build): flowing tables across pages with repeated headers and kept totals` |
| Size | L (≈ 500–700 lines of code with unit tests) |
| PR | — |

## Goal

Split tables across pages with their header rows repeated, rows kept whole, and the totals kept with the rows above them. Hacker News commenters name this in the page-control requests (about 345 comments), and the table failure is the classic one: "Invoices: Totals get pushed to a new page with no repeated <thead> header. This is a classic failure of CSS table rendering across page breaks." ([44968804](https://news.ycombinator.com/item?id=44968804)) Another asked for "repeating headers per page, and not having any problems with rows printing half on one page, and half on the next page" ([15410166](https://news.ycombinator.com/item?id=15410166)). The [`flow-layout` spike](../../../spikes/flow-layout/README.md) showed the same gap in our builder: its equal-width columns clipped the invoice's descriptions silently, and each page's part of a table became its own `Table` element. Footnotes and contents pages, once part of this todo, are now [T4h](./T4h-flow-notes-contents-outline.md).

## Context

- [T4b](./T4b-flow-layout-core.md): the flow model, the paginator, page templates, `flow-content-too-large` and `flow-keep-broken`. [T4f](../ongoing/T4f-flow-builder-groundwork.md): elements across pages, measuring without drawing (for automatic column widths) and artifact subtypes. [T4g](./T4g-flow-styled-spans.md): spans in cells.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.6 (page numbers, a contents page with page numbers, footnotes and repeated table headers) and §2.4 (pagination from HTML); the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- ISO 32000-1 §14.8.4 (the PDF 1.7 standard types: `Table`, `THead`, `TBody`, `TFoot`, `TR`, `TH`, `TD`) and §14.8.5.7 (table attributes: `/Scope`, `/ColSpan`). The Matterhorn Protocol's checkpoint 15 (tables).
- `laminapdf-build`'s tagging: `TagKind` (`src/tag/mod.rs`) has 25 kinds and lacks `THead`, `TBody` and `TFoot`; `Tag::custom` writes a standard type's name without a role map; `TableScope` and `Tag::with_scope` exist. A new `TagKind` also changes `laminapdf-wasm`'s name table (`crates/laminapdf-wasm/src/build_dto.rs`) and `packages/sdk/src/generated/builder-enums.ts`, which `cargo xtask gen boundary` writes.
- `StrokeStyle` for rules.
- Extraction: L3b's ruled tables and [T3c](./T3c-tables-across-pages.md), which joins tables that continue across pages. The three-page table built here is an input T3c can use.
- How repeated header rows are tagged: the copies drawn on later pages are artifacts, so the structure tree holds the header once. The PDF Association's *Tagged PDF Best Practice Guide: Syntax* recommends this; cite its clause in the code comment.

## Scope

**In scope**

- **Tables** (`Block::Table`):
  - Columns: fixed widths in points, fractions of the remaining width, or automatic (each column's widest content, scaled down to fit).
  - Header rows repeat at the top of every page the table continues on. Body rows are kept whole. Footer rows (totals) come at the end and stay with the last body row, so they never start a page alone.
  - Cells hold paragraphs with spans, aligned to the start, centre or end (numbers to the end), and to the top, middle or bottom; padding; column spans.
  - Rules as stroke styles (the frame, under the header, between rows) and a header background.
  - Tags: one `Table` element across all its pages, with `THead`, `TBody`, `TFoot`, `TR`, `TH` with `/Scope /Column`, `TD` and `/ColSpan`. Header rows drawn again on later pages are `Pagination` artifacts, and rules and backgrounds `Layout` artifacts.
- **New `TagKind`s:** `THead`, `TBody` and `TFoot`, added to `ALL`, `as_str` and the name lookup, `laminapdf-wasm`'s name table, and `cargo xtask gen boundary`'s output.
- **Examples and goldens:** `flowing_report.rs` (from T4b) gains a table across pages, and its goldens are rewritten. A three-page invoice table (60 rows and a totals footer) becomes the build golden `fixtures/golden/flow-table.pdf` (new), written by `laminapdf-build`'s `examples/golden.rs`.

**Out of scope** (and which todo covers it)

- Rows split across pages: a row taller than the page's body fails with `flow-content-too-large` (T4b's code). Row spans and nested tables: later.
- Footnotes, a contents page and the outline: [T4h](./T4h-flow-notes-contents-outline.md).
- Markdown tables: [T4d](./T4d-markdown-documents.md). The JSON form: [T4e](./T4e-json-documents-and-templates.md); TypeScript: [T4j](./T4j-documents-in-typescript.md).
- Joining tables across pages in extraction: [T3c](./T3c-tables-across-pages.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [ ] Read ADR 0032 §3–§4, T4b's `flow/` modules, T4f's tagging and the ISO clauses in Context
- [ ] `flow/table.rs` (new): column widths, cell layout at the column's width (T4b's measurement), row heights, and the table's pieces as the paginator sees them: a header that repeats, body rows that can't split, and a footer that keeps with the last body row
- [ ] `flow/paginate.rs`: tables (a header drawn again on each later page; a row that doesn't fit starts the next page; the footer kept with the last body row)
- [ ] `flow/emit.rs`: the table tags, one `Table` element across pages (T4f); header rows on later pages inside `Pagination` artifacts, rules and backgrounds inside `Layout` artifacts
- [ ] `src/tag/mod.rs`: the new `TagKind`s, then `laminapdf-wasm`'s name table and `cargo xtask gen boundary`
- [ ] The example and goldens: `cargo xtask golden --update`, `cargo xtask render-golden --update`, the facts in `fixtures/build/facts.json` and the structure facts in `fixtures/build/structure.json` (`tools/fixtures/build_facts.py`, `tools/fixtures/structure_facts.py`)
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] The 60-row table runs over three pages with its header row at the top of each; no row's text is on two pages; the totals footer is on the same page as the last body row
- [ ] The table is one `Table` element with one `THead`, and its `TH` cells have `/Scope /Column`; the header rows on pages 2 and 3 are inside `/Artifact` marked content, and extraction (`Document::extract`) reads that header once
- [ ] The example passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`, and matches its byte golden on two runs in a row
- [ ] Automatic column widths fit the invoice's descriptions without clipping, and are the same on every page of the table
- [ ] `cargo xtask gen boundary --check` passes

## Post-merge checks

None.

## Tests

- Unit tests in `flow/table.rs` (column widths, spans, row heights).
- `crates/laminapdf-build/tests/flow_tables.rs` (new): the three-page table, its structure and its artifacts, read back with `laminapdf-content`'s marked-content events.
- `crates/laminapdf/tests/flow.rs` (T4b's): the table read once by extraction.
- Goldens: the updated `fixtures/golden/examples/flowing_report.pdf` with its render goldens and facts, and `fixtures/golden/flow-table.pdf` (new).

## Docs to update

- [ ] The "Pages that flow" guide (T4b's): tables
- [ ] `laminapdf-build`'s README; `CLAUDE.md`'s Building API paragraph
- [ ] `fixtures/README.md`: the new golden's row
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Automatic column widths** can make long tables jump between widths. Measure the widths once over the whole table, not per page.
- **Repeated header rows and extraction.** If a reader treats the artifact copies as content, it sees the header three times. Check `laminapdf-extract` and pdf.js on the golden.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
