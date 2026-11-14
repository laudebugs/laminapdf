# E4g — Cross-viewer appearance parity suite

| Field | Value |
| --- | --- |
| ID | `E4g` |
| Status | `ready` |
| Plan | [E4 plan §10](../../plans/editor/e4-markup-and-drawing-annotations.md#10-tests) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4c](../done/E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E4e](./E4e-freetext-callouts.md), [X3a](../done/X3a-nightly-lane.md) |
| ADRs | Follows [0004](../../decisions/0004-render-backend.md), [0018](../../decisions/0018-test-strategy.md) |
| Branch | `test/e4g-appearance-parity-suite` |
| PR title | `test(annots): cross-viewer appearance parity` |
| Size | M (200–400): a matrix generator, a three-renderer harness and the nightly wiring |
| PR | — |

## Goal

The full-editor to-do "Round-trip tests: appearance identical in pdf.js and PDFium" as a standing suite: every subtype × property combination rendered by pdf.js, PDFium and hayro and compared with the goldens, run nightly, plus annotations on corpus files moved and saved again.

## Context

- [E4 plan §10](../../plans/editor/e4-markup-and-drawing-annotations.md#10-tests)
- [ADR 0018](../../decisions/0018-test-strategy.md): the differential tier and the nightly lane
- [04 §2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter)

## Scope

**In scope**

- A matrix generator: subtype × colour × opacity × border × page rotation, through the façade
- A harness that renders each case with pdf.js, PDFium and hayro and compares it with the golden, with a threshold per renderer
- The corpus edit test: every annotation of the pdf.js corpus's `annotation-*.pdf` files moved by 1 pt and saved, then checked by qpdf and both renderers
- The nightly lane's job and its report

**Out of scope** (and which todo covers it)

- The interface and the desktop → [E4f](E4f-annotation-tools-ui.md), [E4h](E4h-desktop-annotations.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.16` generate appearance streams (cross-cutting)
- **Crates and packages:** tests and `tools/xtask` lane config
- **`SaveConstraint`:** incremental saves on `Free` fixtures
- **Web vs desktop:** n/a (core suite)
- **Server features:** none
- **Capability-matrix rows to flip:** `D.16` → `round-trip` (core, pdf)

## Tasks

- [ ] Matrix generator
- [ ] Harness and thresholds per renderer
- [ ] Corpus edit test
- [ ] Nightly lane wiring

## Acceptance criteria

- [ ] The suite passes locally and through `workflow_dispatch` on the PR branch; a failure uploads diff images as artefacts
- [ ] Every subtype of the round is in the matrix, on pages turned 0°, 90°, 180° and 270°

## Post-merge checks

- [ ] The suite passes in the first scheduled nightly run on `main` (link it in a follow-up PR or the done todo)

## Tests

The suite itself.

## Docs to update

- `docs/guidelines` testing note if the thresholds are new
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
