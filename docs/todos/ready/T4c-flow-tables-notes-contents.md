# T4c — Flowing layout: tables with repeated headers, footnotes, a contents page with page numbers

| Field | Value |
| --- | --- |
| ID | `T4c` |
| Status | `ready` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4b](./T4b-flow-layout-core.md) |
| ADRs | Implements ADR 0032 (written by [T4a](./T4a-adr-0032-layout-engine.md)): tables, notes, contents and the pass limit; follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space, 1-based page numbers in what users read) and [0026](../../decisions/0026-error-and-diagnostic-contract.md) (codes) |
| Branch | `feat/t4c-flow-tables-notes-contents` |
| PR title | `feat(build): flowing tables with repeated headers, footnotes and a contents page` |
| Size | XL (≈ 1,000–1,300 lines): [T4a](./T4a-adr-0032-layout-engine.md) splits it into todos of at most L once ADR 0032 decides the design |
| PR | — |

> **Written to the expected decision.** Like [T4b](./T4b-flow-layout-core.md), this todo assumes ADR 0032 chooses an engine of our own; T4a rewrites it if the ADR decides otherwise.

## Goal

Add the three page features that need more than one pass or more than one region of the page. Tables split across pages with their header rows repeated, rows kept whole, and the totals kept with the rows above them. Footnotes sit at the foot of the page that refers to them. A contents page lists each heading with the page it landed on and a link to it. Hacker News commenters name each of these in the page-control requests (about 345 comments), and the table failure is the classic one: "Invoices: Totals get pushed to a new page with no repeated <thead> header. This is a classic failure of CSS table rendering across page breaks." ([44968804](https://news.ycombinator.com/item?id=44968804)) Another asked for "repeating headers per page, and not having any problems with rows printing half on one page, and half on the next page" ([15410166](https://news.ycombinator.com/item?id=15410166)).

## Context

- [T4b](./T4b-flow-layout-core.md): the flow model, the paginator, page templates, elements that continue across pages, `flow-content-too-large` and `flow-keep-broken`. ADR 0032: the decision and the limit on layout passes.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.6 (page numbers, a contents page with page numbers, footnotes and repeated table headers) and §2.4 (pagination from HTML); the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- ISO 32000-1 §14.8.4 (the PDF 1.7 standard types: `Table`, `THead`, `TBody`, `TFoot`, `TR`, `TH`, `TD`, `TOC`, `TOCI`, `Note`, `Reference`) and §14.8.5.7 (table attributes: `/Scope`, `/ColSpan`); ISO 32000-2 §12.3.2 (destinations) and §12.3.3 (the outline). The Matterhorn Protocol's checkpoints 15 (tables) and 19 (notes and references; 19-003 and 19-004 ask for a unique `/ID` on each `Note`).
- `laminapdf-build`'s tagging: `TagKind` (`src/tag/mod.rs`) has 25 kinds and lacks `TOC`, `TOCI`, `Reference`, `THead`, `TBody` and `TFoot`; `Tag::custom` writes a standard type's name without a role map; `TableScope` and `Tag::with_scope` exist. A new `TagKind` also changes `laminapdf-wasm`'s name table (`crates/laminapdf-wasm/src/build_dto.rs`) and `packages/sdk/src/generated/builder-enums.ts`, which `cargo xtask gen boundary` writes.
- Navigation: `Builder::destination`, `Builder::outline` with `OutlineEntry`, `LinkTarget::Named`, and `StrokeStyle` for rules.
- Extraction: L3b's ruled tables and [T3c](../ongoing/T3c-tables-across-pages.md), which joins tables that continue across pages. The three-page table built here is an input T3c can use.
- How repeated header rows are tagged: the copies drawn on later pages are artifacts, so the structure tree holds the header once. The PDF Association's *Tagged PDF Best Practice Guide: Syntax* recommends this; cite its clause in the code comment.

## Scope

**In scope**

- **Tables** (`Block::Table`):
  - Columns: fixed widths in points, fractions of the remaining width, or automatic (each column's widest content, scaled down to fit).
  - Header rows repeat at the top of every page the table continues on. Body rows are kept whole. Footer rows (totals) come at the end and stay with the last body row, so they never start a page alone.
  - Cells hold paragraphs with spans, aligned to the start, centre or end (numbers to the end), and to the top, middle or bottom; padding; column spans.
  - Rules as stroke styles (the frame, under the header, between rows) and a header background.
  - Tags: one `Table` element across all its pages, with `THead`, `TBody`, `TFoot`, `TR`, `TH` with `/Scope /Column`, `TD` and `/ColSpan`. Header rows drawn again on later pages are `Pagination` artifacts, and rules and backgrounds `Layout` artifacts.
- **Footnotes:** a note inline in a paragraph, with markers numbered through the document and drawn raised (T4b's span rise). The note's body goes at the foot of the page that holds its marker, above the footer, under a short rule (a `Layout` artifact). When the note doesn't fit, the line holding its marker moves to the next page. Tags: the marker is a `Reference` element inside its paragraph, and the body a `Note` with a unique `/ID`, after that paragraph in structure order.
- **A contents page** (`Block::Contents`): the headings up to a chosen level, each entry with dot leaders (`Layout` artifacts), the page number at the end side and a link to a named destination the flow makes for each heading; tagged `TOC` and `TOCI`, each entry holding its `Link`. The numbers come from repeated layout: lay out, record where each heading lands, and lay out again until no number changes, within the pass limit ADR 0032 sets (proposal: four). A flow that is still changing then fails with `flow-layout-unstable` (new, `buildErrors`, with its `docs/errors/` page).
- **The outline (bookmarks)** made from the headings, nested by level, on by default, with each item pointing at its heading's destination.
- **New `TagKind`s:** `TOC`, `TOCI`, `Reference`, `THead`, `TBody` and `TFoot`, added to `ALL`, `as_str` and the name lookup, `laminapdf-wasm`'s name table, and `cargo xtask gen boundary`'s output.
- **Examples and goldens:** `flowing_report.rs` (from T4b) gains a contents page, a table across pages and footnotes, and its goldens are rewritten. A three-page invoice table (60 rows and a totals footer) becomes the build golden `fixtures/golden/flow-table.pdf` (new), written by `laminapdf-build`'s `examples/golden.rs`.

**Out of scope** (and which todo covers it)

- Rows split across pages: a row taller than the page's body fails with `flow-content-too-large` (T4b's code). Row spans, nested tables, endnotes, links from a note back to its marker, an index, and cross-references such as "see page 12": later, as ADR 0032 records.
- Markdown tables and footnotes: [T4d](./T4d-markdown-documents.md). The JSON form and TypeScript: [T4e](./T4e-json-documents-and-templates.md).
- Joining tables across pages in extraction: [T3c](../ongoing/T3c-tables-across-pages.md).

## Tasks

- [ ] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [ ] Read ADR 0032, T4b's `flow/` modules, `src/tag/` and the ISO clauses in Context
- [ ] `flow/table.rs` (new): column widths, cell layout at the column's width (T4b's measurement), row heights, and the table's pieces as the paginator sees them: a header that repeats, body rows that can't split, and a footer that keeps with the last body row
- [ ] `flow/paginate.rs`: tables (a header drawn again on each later page; a row that doesn't fit starts the next page), and room at the foot of a page for its notes (a line whose note doesn't fit moves to the next page)
- [ ] `flow/notes.rs` (new): note numbering, markers as raised spans, the bodies and the rule at the foot of the page
- [ ] `flow/contents.rs` (new): entries, leaders and right-aligned numbers; the pass loop over the whole layout, with the limit and `flow-layout-unstable`; a named destination per heading (`Builder::destination`) and the outline (`Builder::outline`)
- [ ] `flow/emit.rs`: the table, note and contents tags; header rows on later pages and the leaders inside `Pagination` and `Layout` artifacts; a `/ID` for each `Note`, unique in the document
- [ ] `src/tag/mod.rs`: the new `TagKind`s, then `laminapdf-wasm`'s name table and `cargo xtask gen boundary`
- [ ] The code `flow-layout-unstable`: its `codes.json` entry, its `docs/errors/` page and `cargo xtask gen codes`
- [ ] The example and goldens: `cargo xtask golden --update`, `cargo xtask render-golden --update`, the facts in `fixtures/build/facts.json` and the structure facts in `fixtures/build/structure.json` (`tools/fixtures/build_facts.py`, `tools/fixtures/structure_facts.py`)
- [ ] The docs (Docs to update)

## Acceptance criteria

- [ ] The 60-row table runs over three pages with its header row at the top of each; no row's text is on two pages; the totals footer is on the same page as the last body row
- [ ] The table is one `Table` element with one `THead`, and its `TH` cells have `/Scope /Column`; the header rows on pages 2 and 3 are inside `/Artifact` marked content, and extraction (`Document::extract`) reads that header once
- [ ] In the example, every footnote marker is on the same page as its note; a marker whose note doesn't fit moves to the next page with its line; every `Note` has a unique `/ID`
- [ ] Every contents entry's number equals the page where its heading's element starts (read from the structure tree), and its link goes to that heading's named destination (`/Names /Dests`)
- [ ] A contents page that grows to two pages, and so pushes every heading down a page, settles within the pass limit; a unit test drives the pass loop with a stub that never settles and gets `flow-layout-unstable`
- [ ] The outline has one item per heading, nested as the headings are, each pointing at its heading's page
- [ ] The example passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`, and matches its byte golden on two runs in a row
- [ ] `cargo xtask gen codes --check` and `cargo xtask gen boundary --check` pass

## Post-merge checks

None.

## Tests

- Unit tests in `flow/table.rs` (column widths, spans, row heights), `flow/notes.rs` (numbering, placement, a moved line) and `flow/contents.rs` (the pass loop with a stub measure).
- `crates/laminapdf-build/tests/flow_tables.rs` (new): the three-page table, its structure and its artifacts, read back with `laminapdf-content`'s marked-content events.
- `crates/laminapdf/tests/flow.rs` (T4b's): notes, contents numbers and links, the outline, and the table read once by extraction.
- Goldens: the updated `fixtures/golden/examples/flowing_report.pdf` with its render goldens and facts, and `fixtures/golden/flow-table.pdf` (new).

## Docs to update

- [ ] The "Pages that flow" guide (T4b's): tables, footnotes and contents
- [ ] `laminapdf-build`'s README; `CLAUDE.md`'s Building API paragraph
- [ ] `fixtures/README.md`: the new golden's row
- [ ] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Automatic column widths** can make long tables jump between widths. Measure the widths once over the whole table, not per page.
- **Notes and keep rules interact.** A moved line can move its note, which can move another line. Bound it as T4b bounds keep chains, and test a page with three notes.
- **Repeated header rows and extraction.** If a reader treats the artifact copies as content, it sees the header three times. Check `laminapdf-extract` and pdf.js on the golden.
- **Note placement in the structure tree.** Placing the `Note` right after its paragraph is the common reading; check it against veraPDF and a screen reader if one is at hand, and record the choice in ADR 0032.
- **Contents that don't start at page 1.** The printed numbers are document page numbers (`{page}`), so they match what a viewer shows without `/PageLabels`. Roman numbers for front matter are later.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
