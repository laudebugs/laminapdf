# E5f — The comment summary

| Field | Value |
| --- | --- |
| ID | `E5f` |
| Status | `ready` |
| Plan | [E5 plan §2.3](../../plans/editor/e5-review-stamps-interchange.md#23-interchange-and-the-summary) ([full-editor §E5](../../plans/full-editor.md#e5-review-stamps-and-interchange)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E2p](../done/E2p-source-documents-and-copier.md), [E5a](../done/E5a-review-stamps-interchange-feature-plan.md), [E5b](./E5b-comment-threads-and-review.md) |
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

- The dialog and opening the summary → [E5i](E5i-interchange-flatten-summary-ui.md)

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
