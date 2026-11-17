# E5f — The comment summary

| Field | Value |
| --- | --- |
| ID | `E5f` |
| Status | `ongoing` |
| Plan | [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2p](../done/E2p-source-documents-and-copier.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5b](../done/E5b-comment-threads-and-review.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0032](../../decisions/0032-flowing-layout-and-document-inputs.md) |
| Branch | `feat/e5f-comment-summary` |
| PR title | `feat(build): the comment summary as a new tagged PDF` |
| Size | L (400–800): two layouts over the flowing layout, and pages copied as Form XObjects |
| PR | — |

## Goal

`comment_summary(&doc, &SummaryOptions)`: a new PDF/UA-1 document listing every comment, with each page drawn beside its comments (the default) or as a list ([E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary)).

## Context

- [E5 plan §2.3 and §10](../../plans/editor/e5-review-stamps-interchange.md#10-decisions-on-the-open-questions)
- **Design (Paper):** [C07 Flatten and comment summary](../../design/paper/comment-review.md#c07-flatten-and-comment-summary)
- [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md): the flowing layout

## Scope

**In scope**

- Pages copied as Form XObjects through `ObjectCopier` (E2p), drawn small with the comments' numbers marked
- The list and beside-its-page layouts over `Builder::flow`, with C07's options: replies, status, empty pages, paper size
- Tagged and checked by the PDF/UA-1 profile; each drawn page a `Figure` whose alternative text names it
- The façade, WASM (text module) and SDK

**Out of scope** (and which todo covers it)

- The dialog and opening the summary → [E5i](../ready/E5i-interchange-flatten-summary-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.15` summarise annotations
- **Crates and packages:** `laminapdf-build`, `laminapdf-model`, `laminapdf`, `laminapdf-wasm`, `@laminapdf/sdk`
- **`Op` variants:** none: the summary is a new document
- **`SaveConstraint`:** none: works on any document, signed or encrypted once opened
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.15` → `round-trip` (core, pdf)

## Tasks

- [ ] Pages as Form XObjects
- [ ] The two layouts
- [ ] The profile check
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new op and function is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] veraPDF passes the summary of `review.pdf` as PDF/UA-1 in both layouts
- [ ] The page beside its comments is vector: its text extracts with pdf.js

## As built

- **Where it lives.** The plan put the summary in `laminapdf-build`; it is split instead. `laminapdf-build` has the general parts: `Builder::page_forms(&source, &[pages])` copies pages as Form XObjects (`page_form.rs`: the page's decoded content within the source's `Limits`, less its marked-content operators, its resources through one `ObjectCopier`, so what several pages share is copied once; `/BBox` the crop box, the source's frame inverted so the copy draws the way the page displays; annotations aren't copied), `Surface::page_form`, `Figure::page` and `Figure::with_mark` (a label in a filled circle over a figure, drawn inside its `Figure` element; `flow/figure.rs`) and `FlowBlock::Beside(figure, blocks)`, a figure at the frame's left with the blocks in the column to its right, 12 points from it, going on in that column on later pages, what follows starting below both (the paginator's `indent` and `clear`). The summary itself is the façade's `Document::comment_summary(&SummaryOptions) -> Document` (`laminapdf/src/summary.rs`, feature `text`), since it reads `comment_threads`, whose quoted text needs the façade's extraction, as the contents page does.
- **Options** follow C07: `SummaryLayout::{BesidePage (default), List}`, `with_comments(ids)` (the panel's "Shown in the Comments panel"), `with_replies`, `with_status`, `with_empty_pages`, `SummaryPaper::{SameAsDocument (default), Size}`, `with_title`, `with_language` (the catalog's `/Lang`; the summary's own words are English) and a required `Font` with fallbacks: PDF/UA-1 needs an embedded font, and the core bundles none. Margins are 12% of the paper's short side, from 18 to 72 points.
- **The layout:** an `H1` title ("Comments on" and the document's title, else "Comment summary"), a count line, then per page an `H2` "Page" and its label, and per comment, numbered in document order, its number and kind, author and date, the quoted text, the note, the grouped annotations' notes, "Status: … · Marked", and the replies (state-setting replies left out) as a list. Beside its page, the page is a `Figure` whose `/Alt` names the page and the comments it marks ("Page 1, with comments 1 to 6 marked"), 42% of the body's width at most, its edge stroked, each comment's number at its rectangle's top-left. A footer "n / N" is a `Pagination` artifact.
- **Fonts the summary can't hold.** A page whose fonts aren't all embedded (`Page::fonts`, new in `laminapdf-model`) would break PDF/UA-1, so it is listed without its drawing and the new build warning `summary-page-not-drawn` (`unsupported`, data `page`) says so. Other font rules of the source's own fonts (widths, encodings) are copied as they are. `fixtures/annotations/review.pdf` now embeds its font (Noto Sans, `tools/fixtures/annotations.py`'s `truetype`), so its page can be drawn; its annotations are unchanged.
- **Tagged sources.** A copied page's `BDC`/`BMC`/`EMC`/`MP`/`DP` (with their operands) are taken out of the copy, since its MCIDs and artifacts would sit inside the summary's `Figure` (veraPDF 7.1-1 and 7.1-2 on the example, whose source is tagged). Forms the page draws keep theirs; none of the fixtures has marked content inside a form.
- **Checks:** veraPDF 1.30.3 (`cargo xtask verapdf --dir`) passes the summary of `review.pdf` as PDF/UA-1 in both layouts, on A4 and on its own 300-point paper, and the example's golden (106 rules, 0 failed each). pdf.js 6.4.299 extracts the drawn page's text: "Quoted words" twice in the beside layout (on the page, then quoted), once in the list. `the_page_beside_its_comments_is_vector_text` checks the same with Lamina PDF's extractor after a save and a reopen.
- **Bindings:** WASM `WasmDocument.commentSummary(options, fonts)` in the text module (`CommentSummaryInit`, fonts as `layOutContentsPage` takes them), the SDK's `doc.commentSummary({ font, … })`, which needs a document opened while the text module was loaded (`text-module-not-loaded` otherwise). No op: nothing in `fixtures/ops/v1/`.
- **Capability D.15** is round-trip in the core column, from the round-trip suite's new summary cases (`laminapdf-save`'s `tests/common/roundtrip_summary.rs`, both layouts; the façade is now a path-only dev-dependency of `laminapdf-save`), which `cargo xtask roundtrip` checked clean in qpdf 12.4.2, pdf.js 6.4.299 and PDFium on 2026-11-17, and in the pdf column (`packages/sdk/test/summary.test.ts`). Deleting annotations, its other half, shipped with E5b.
- **Example and docs:** `comment_summary` / `comment-summary`, summarising `review_comments`' answered review, byte-identical in Rust and TypeScript, with its render goldens and docs preview, on the docs site's Print a comment summary recipe. The worker client doesn't serve `commentSummary`: it returns a new document, as `extractPages` does.

## Post-merge checks

None.

## Tests

- The golden summary of `review.pdf`
- Unit tests for numbering, replies and empty pages

## Docs to update

- API docs: the comment summary
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)
- [full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange) and the [E5 plan §7](../../plans/editor/e5-review-stamps-interchange.md#7-todos): point this todo's link at `done/`

## Risks and open questions

- A page copied as a form keeps its resources; a very large page may need `Limits` checks, which the PR measures

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
