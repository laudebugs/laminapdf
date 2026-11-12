# Q4c — Guides for the viewer, rendering pages, and reveal and redaction from code

| Field | Value |
| --- | --- |
| ID | `Q4c` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0004](../../decisions/0004-render-backend.md) (render backends), [ADR 0009](../../decisions/0009-web-client-architecture.md) (the viewer) |
| Branch | `docs/q4c-viewer-render-redaction` |
| PR title | `docs: guides for the viewer, rendering pages, and checking what a file hides` |
| Size | M |
| PR | [#463](https://github.com/laudebugs/laminapdf/pull/463) |

## Goal

The viewer and render packages appear in the docs only as rows in the install table, and reveal and redaction checks only on the command-line pages. A developer who wants to show a PDF in a web page, render a page to an image, or check what a file hides from code has nothing to start from. This todo writes those three guides.

## Context

- [Docs research: recommendation 2](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [audit](../../research/docs-site/developer-docs/current_docs_site.md)'s missing topics.
- `@laminapdf/viewer` (`createViewer`, `ViewerClient`), `@laminapdf/render-pdfium`, `@laminapdf/render-hayro`, and `laminapdf-render` in Rust.
- `Document::reveal`, `redaction_warnings` and `hidden_text` in `laminapdf`; `laminapdf reveal` and `check` on the command line.
- The [docs diagrams plan](../../plans/docs-diagrams.md#2-where-a-diagram-helps) holds back a diagram of the viewer's parts until its API is stable.

## Scope

**In scope**

- `guides/show-a-pdf`: the viewer in a web page, with its worker and backend.
- `guides/render-pages`: a page to an image in the browser and in Rust.
- `guides/check-what-a-file-hides`: reveal and redaction warnings from code, linked from the command-line pages.
- Tested examples for each, in the languages each API has.

**Out of scope**

- The reader app (`app.laminapdf.com`).
- Extraction (Q4b).

## Tasks

- [ ] Confirm with the viewer's owners which of its API is stable enough to document; mark the rest as preview.
- [ ] Write the examples and their tests.
- [ ] Write the three guides and place them in the sidebar and llms.txt.

## Acceptance criteria

- [ ] Each guide starts from a tested example and shows its result.
- [ ] The install table links each package to its guide.

## Post-merge checks

None.

## Tests

The new examples' tests; the viewer guide's example in the browser test lane if it needs a browser.

## Docs to update

The guides, the install page, the command-line pages' links.

## Risks and open questions

- The viewer's API is still moving; a guide written now may need a follow-up. Document only what its owners call stable.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
