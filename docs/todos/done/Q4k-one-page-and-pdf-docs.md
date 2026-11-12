# Q4k — Each area on one page, and the whole guide as a PDF

| Field | Value |
| --- | --- |
| ID | `Q4k` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0032](../../decisions/0032-flowing-layout-and-document-inputs.md) (Markdown to PDF) |
| Branch | `feat/q4k-one-page-and-pdf-docs` |
| PR title | `feat(docs): one-page areas and the guide as a PDF built by Lamina PDF` |
| Size | M |
| PR | [#472](https://github.com/laudebugs/laminapdf/pull/472) |

## Goal

Hacker News commenters ask again and again for docs they can read offline or on one page (252 comments, about 7 in 8 on topic), and praise sites like Lua's that give them. The docs already have a Markdown twin of every page. This todo adds a page per area that holds all of its pages, and the whole guide as one PDF, built by `laminapdf build` from the Markdown twins: a demonstration of the library as well as a convenience.

## Context

- [Docs research: recommendation 9](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [Hacker News note](../../research/docs-site/developer-docs/hacker_news.md).
- `plugins/markdown.mjs` (the twins), `crates/laminapdf/examples/markdown_to_pdf.rs` and `laminapdf build` (T4d, T4i).

## Scope

**In scope**

- A route per sidebar group that renders its pages in order on one page.
- A PDF of the guide (not the TypeDoc reference or the code pages), built in the docs build and linked from the home page and the footer.

**Out of scope**

- Printed styles for the HTML pages.

## Tasks

- [x] Add the one-page routes from the sidebar's order; keep them out of search so results point at the single pages.
- [x] Build the PDF from the twins with `laminapdf build`; add a cover, contents and the version.
- [x] Link both; check the PDF with the profile checks the examples use.

## Acceptance criteria

- [x] Each area has a one-page view.
- [x] The guide PDF builds in the docs build and opens in pdf.js and PDFium.

## Post-merge checks

None.

## Tests

The docs build; a test that the PDF exists, has an outline and passes `qpdf --check`.

## Docs to update

The home page, the docs README.

## Risks and open questions

- Diagrams are SVG in the HTML; the PDF needs them as images or as their text descriptions until Markdown input takes SVG.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
