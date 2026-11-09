# T4h — Flowing layout: footnotes, a contents page with page numbers, and the outline

| Field | Value |
| --- | --- |
| ID | `T4h` |
| Status | `done` |
| Plan | [trust and proof §T4](../../plans/trust-and-proof.md#t4-pages-that-flow) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [T4b](./T4b-flow-layout-core.md) |
| ADRs | Implements [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md) [§3](../../decisions/0032-flowing-layout-and-document-inputs.md#3-the-layout-model) (contents pages and the pass limit) and [§4](../../decisions/0032-flowing-layout-and-document-inputs.md#4-tagging) (tagging); follows [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (1-based page numbers in what users read) and [0026](../../decisions/0026-error-and-diagnostic-contract.md) (codes) |
| Branch | `feat/t4h-flow-notes-contents-outline` |
| PR title | `feat(build): footnotes, a contents page with page numbers, and the outline in flowing layout` |
| Size | L (≈ 550–750 lines of code with unit tests) |
| PR | [#351](https://github.com/laudebugs/laminapdf/pull/351) |

## Goal

Add the two page features that need more than one region of the page or more than one pass. Footnotes sit at the foot of the page that refers to them. A contents page lists each heading with the page it landed on and a link to it, and the outline (bookmarks) mirrors the headings. Hacker News commenters name both in the page-control requests (about 345 comments). This was part of T4c until [T4a](../done/T4a-adr-0032-layout-engine.md) split T4c to keep each todo under 800 lines; tables across pages stay in [T4c](../done/T4c-flow-tables-notes-contents.md).

## Context

- [T4b](../done/T4b-flow-layout-core.md): the flow model, the paginator, page templates, `flow-content-too-large` and `flow-keep-broken`. [T4g](../done/T4g-flow-styled-spans.md): span rises, for raised markers. [T4f](../done/T4f-flow-builder-groundwork.md): elements across pages and artifact subtypes.
- [ADR 0032 §3](../../decisions/0032-flowing-layout-and-document-inputs.md#3-the-layout-model): a contents page lays out again until no page number changes, at most four passes, then fails with `flow-layout-unstable`.
- Evidence: the [generation note](../../research/hacker-news/notes/generation.md) §1.6 (a contents page with page numbers, footnotes); the report's [R5](../../research/hacker-news/pdf-users-on-hacker-news.md#r5-decide-for-flow-layout-then-accept-markdown-and-json).
- ISO 32000-1 §14.8.4 (the PDF 1.7 standard types `TOC`, `TOCI`, `Note`, `Reference`); ISO 32000-2 §12.3.2 (destinations) and §12.3.3 (the outline). The Matterhorn Protocol's checkpoint 19 (notes and references; 19-003 and 19-004 ask for a unique `/ID` on each `Note`).
- `laminapdf-build`: `TagKind` (`src/tag/mod.rs`) lacks `TOC`, `TOCI` and `Reference`; a new `TagKind` also changes `laminapdf-wasm`'s name table (`crates/laminapdf-wasm/src/build_dto.rs`) and `packages/sdk/src/generated/builder-enums.ts`, which `cargo xtask gen boundary` writes. Navigation: `Builder::destination`, `Builder::outline` with `OutlineEntry`, `LinkTarget::Named`.

## Scope

**In scope**

- **Footnotes:** a note inline in a paragraph, with markers numbered through the document and drawn raised (T4g's rise). The note's body goes at the foot of the page that holds its marker, above the footer, under a short rule (a `Layout` artifact). When the note doesn't fit, the line holding its marker moves to the next page. Tags: the marker is a `Reference` element inside its paragraph, and the body a `Note` with a unique `/ID`, after that paragraph in structure order.
- **A contents page** (`Block::Contents`): the headings up to a chosen level, each entry with dot leaders (`Layout` artifacts), the page number at the end side and a link to a named destination the flow makes for each heading; tagged `TOC` and `TOCI`, each entry holding its `Link`. The numbers come from repeated layout: lay out, record where each heading lands, and lay out again until no number changes, at most four passes. A flow still changing then fails with `flow-layout-unstable` (new, `buildErrors`, with its `docs/errors/` page).
- **The outline (bookmarks)** made from the headings, nested by level, on by default, with each item pointing at its heading's destination.
- **New `TagKind`s:** `TOC`, `TOCI` and `Reference`, added to `ALL`, `as_str` and the name lookup, `laminapdf-wasm`'s name table, and `cargo xtask gen boundary`'s output.
- **The example:** `flowing_report.rs` (from T4b) gains a contents page and footnotes, and its goldens are rewritten.

**Out of scope** (and which todo covers it)

- Tables across pages: [T4c](../done/T4c-flow-tables-notes-contents.md).
- Endnotes, links from a note back to its marker, an index, cross-references such as "see page 12", and roman numbers for front matter: later.
- Markdown footnotes: [T4d](../ongoing/T4d-markdown-documents.md).

## Tasks

- [x] Claim via its work PR; a fresh worktree off `main`. Load `/rust-best-practices`
- [x] Read ADR 0032 §3–§4, T4b's `flow/` modules and the ISO clauses in Context
- [x] `flow/notes.rs` (new): note numbering, markers as raised spans, the bodies and the rule at the foot of the page
- [x] `flow/paginate.rs`: room at the foot of a page for its notes (a line whose note doesn't fit moves to the next page)
- [x] `flow/contents.rs` (new): entries, leaders and end-aligned numbers; the pass loop over the whole layout, with the limit and `flow-layout-unstable`; a named destination per heading (`Builder::destination`) and the outline (`Builder::outline`)
- [x] `flow/emit.rs`: the note and contents tags; leaders inside `Layout` artifacts; a `/ID` for each `Note`, unique in the document
- [x] `src/tag/mod.rs`: the new `TagKind`s, then `laminapdf-wasm`'s name table and `cargo xtask gen boundary`
- [x] The code `flow-layout-unstable`: its `codes.json` entry, its `docs/errors/` page and `cargo xtask gen codes`
- [ ] The example's goldens (structure facts deferred; see As built): `cargo xtask golden --update`, `cargo xtask render-golden --update`, and the structure facts in `fixtures/build/structure.json` (`tools/fixtures/structure_facts.py`)
- [x] The docs (Docs to update)

## Acceptance criteria

- [x] In the example, every footnote marker is on the same page as its note; a marker whose note doesn't fit moves to the next page with its line; every `Note` has a unique `/ID`
- [x] Every contents entry's number equals the page where its heading's element starts (read from the structure tree), and its link goes to that heading's named destination (`/Names /Dests`)
- [x] A contents page that grows to two pages, and so pushes every heading down a page, settles within four passes; a unit test drives the pass loop with a stub that never settles and gets `flow-layout-unstable`
- [x] The outline has one item per heading, nested as the headings are, each pointing at its heading's page
- [x] The example passes `cargo xtask verapdf` under PDF/UA-1 and `qpdf --check`, and matches its byte golden on two runs in a row
- [x] `cargo xtask gen codes --check` and `cargo xtask gen boundary --check` pass

## Post-merge checks

None.

## Tests

- Unit tests in `flow/notes.rs` (numbering, placement, a moved line, a page with three notes) and `flow/contents.rs` (the pass loop with a stub measure).
- `crates/laminapdf/tests/flow.rs` (T4b's): notes, contents numbers and links, and the outline.
- Goldens: the updated `fixtures/golden/examples/flowing_report.pdf` with its render goldens and facts.

## Docs to update

- [x] The "Pages that flow" guide (T4b's): footnotes and contents
- [x] `laminapdf-build`'s README; `CLAUDE.md`'s Building API paragraph
- [x] The trust and proof plan's §10 to-do for T4

## Risks and open questions

- **Notes and keep rules interact.** A moved line can move its note, which can move another line. Bound it as T4b bounds keep chains, and test a page with three notes.
- **Note placement in the structure tree.** Placing the `Note` right after its paragraph is the common reading; check it against veraPDF and a screen reader if one is at hand.
- **Contents that don't start at page 1.** The printed numbers are document page numbers (`{page}`), so they match what a viewer shows without `/PageLabels`.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Footnotes** (`Paragraph::note(body)`, `flow/notes.rs`): an `Inline::Note` in the paragraph. Markers are numbered through the flow and drawn at 65 % of the size of the text before them, raised by 35 % of it. `Source::numbered` records each marker's byte and note, and `Items` turns each body into a source that starts with its number. A paragraph's first lines count with their notes when a heading is kept with it, and a table's start counts its header rows' notes. `flow-content-too-large` gains `what: "footnote"` for a line or row whose notes fit no page. A line fits only with the notes it cites: the paginator keeps their height at the page's foot (12 points above the first note, holding a 72-point rule as a `Layout` artifact, then 2 points between notes), so a line whose notes don't fit moves to the next page with them. Widows and orphans then apply as before. Notes cited in table cells work the same way for their row; a note inside a note's body is left out.
- **Deviation: where the `Note` element goes.** The todo puts it after the citing paragraph. It is inside the citing element instead, after that element's content on the page: a `Note` after an `LBody` or a `TD` would break `LI` and `TR` nesting, and PDF/UA accepts `Note` inside `P`. Its `/ID` comes from the builder's existing `Note` numbering (Matterhorn 19-003, 19-004). The marker is a `Reference` inside the paragraph. A note's `Note` element and a contents entry's `TOCI` take the language of their text. `{section}` reads a heading's text without its notes.
- **Contents** (`FlowBlock::Contents(Contents)`, `flow/contents.rs`): an entry per heading up to `with_levels` (2 by default), indented `with_indent` (12 points) per level. The title and the page number are each a link to the heading's named destination. A `TOCI` holds the two `Link`s, and the entries share one `TOC`. The number sits at the right edge on the entry's last line, the dots run to it on a 4-point grid counted from the body's right edge, so all entries' dots line up (a `Layout` artifact), and each entry keeps three times its text size free at the line's end for the number. Entries are laid out left to right.
- **The passes** (`contents::settle`): the flow is laid out with the numbers of the last layout, starting from the flow's first page for every heading, until the numbers it finds equal those it printed, at most four layouts (`flow-layout-unstable` after). A flow without a contents block is laid out once. The paginator records where each item starts (`Laid::starts`), and the numbers are those pages plus the pages before the flow. Each number has a fixed room, so the contents page's length never depends on its numbers and the second layout always settles: `flow-layout-unstable` is a guard against a fault, so its kind is `internal`, not `usage` as the todo implied. `Items` is built once and only the number sources are replaced between layouts (`Items::set_numbers`); the measurements (`paginate::Measures`) are kept, minus those sources, so the text is shaped once.
- **Destinations and the outline:** every heading gets a named destination `flow<first page>-h<n>` with `/XYZ` at its top. The outline (`Flow::with_outline`, on by default) gets an item per heading, nested by level and pointing at that destination. Turning the outline on by default rewrote the goldens of `flowing_invoice.pdf` and `flow-table.pdf`, whose page content is unchanged.
- **Tag kinds** `TOC`, `TOCI` and `Reference` (the last of `ALL`), out of the role map's list of other standard types; `cargo xtask gen boundary` added them to `@laminapdf/sdk`'s names. The code `flow-layout-unstable` (`buildErrors`, no data fields) has its `docs/errors/` page.
- **The example** (`flowing_report.rs`, 99 lines): a contents page on the cover, under a title that is now a paragraph so that the contents lists only the sections, and a footnote on each section's fourth paragraph. On the second paragraph the note moved the Hindi paragraph off its page break, which T4b's test needs. The report is 13 pages.
- **Tests:** `flow/contents.rs` (the pass loop settling and never settling, the outline's nesting), `flow/tests.rs` (a line moving with its note, three notes on a page, numbering), `crates/laminapdf-build/tests/flow_notes.rs` (markers and notes on the same pages, unique `/ID`s, contents numbers equal to the headings' pages from the structure tree, destinations, a two-page contents page that settles, the outline), and the façade's `flow.rs` (the example's contents, bookmarks and footnotes).
- **Deviation: structure facts.** As in T4b and T4c, `fixtures/build/structure.json` has no entry for the new golden: its script needs the pinned qpdf, which this cloud session lacks.
