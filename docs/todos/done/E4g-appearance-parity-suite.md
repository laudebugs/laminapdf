# E4g — Cross-viewer appearance parity suite

| Field | Value |
| --- | --- |
| ID | `E4g` |
| Status | `done` |
| Plan | [E4 plan §10](../../plans/editor/e4-markup-and-drawing-annotations.md#10-tests) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md), [E4c](./E4c-appearances-markup-notes-caret.md), [E4d](./E4d-appearances-ink-and-shapes.md), [E4e](./E4e-freetext-callouts.md), [X3a](./X3a-nightly-lane.md) |
| ADRs | Follows [0004](../../decisions/0004-render-backend.md), [0018](../../decisions/0018-test-strategy.md) |
| Branch | `test/e4g-appearance-parity-suite` |
| PR title | `test(annots): cross-viewer appearance parity` |
| Size | M (200–400): a matrix generator, a three-renderer harness and the nightly wiring |
| PR | [#693](https://github.com/laudebugs/laminapdf/pull/693) |

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

- The interface and the desktop → [E4f](../done/E4f-annotation-tools-ui.md), [E4h](../ready/E4h-desktop-annotations.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.16` generate appearance streams (cross-cutting)
- **Crates and packages:** tests and `tools/xtask` lane config
- **`SaveConstraint`:** incremental saves on `Free` fixtures
- **Web vs desktop:** n/a (core suite)
- **Server features:** none
- **Capability-matrix rows to flip:** `D.16` → `round-trip` (core, pdf)

## Tasks

- [x] Matrix generator
- [x] Harness and thresholds per renderer
- [x] Corpus edit test
- [x] Nightly lane wiring

## Acceptance criteria

- [x] The suite passes locally and through `workflow_dispatch` on the PR branch; a failure uploads diff images as artefacts
- [x] Every subtype of the round is in the matrix, on pages turned 0°, 90°, 180° and 270°

## Post-merge checks

- [ ] The suite passes in the first scheduled nightly run on `main` (link it in a follow-up PR or the done todo)

## Tests

The suite itself.

## Docs to update

- `docs/guidelines` testing note if the thresholds are new
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Flattening's render tests ([E5e](../done/E5e-flatten-and-summary.md): `laminapdf-render-pdfium`'s and `laminapdf-render`'s `tests/flatten.rs`) compare pages before and after with SSIM ≥ 0.99, the render goldens' threshold, for want of this todo's. When this todo sets its per-renderer thresholds, recheck those tests against them.

## As built

- **The matrix:** 16 pages, 400 pt square, so every turn shares one layout: 4 property sets (red, opaque, solid 1 pt; blue, 50 %, dashed 2 pt; green, opaque, cloudy; grey, 50 %, solid 4 pt) on pages turned 0°, 90°, 180° and 270°, each page holding all 14 marks of the round. Built through the façade by `tools/render/src/annots_matrix.rs`, the same bytes every time.
- **pdf.js renders pixels** for this suite only: `pdfjs-dist` 6.4.299's legacy build on `@napi-rs/canvas` 1.0.13 (MIT), pinned in `tools/render`'s own pnpm project. It stays a test oracle, not a backend (ADR 0004).
- **Thresholds** (`fixtures/render-golden/annots-parity.toml`): each engine against its goldens exactly (0 pixels, as measured); hayro within 0.8 % of PDFium and pdf.js within 0.9 % (measured at worst 0.766 % and 0.855 %, SSIM ≥ 0.985), worst on turned pages, where PDFium keeps a note's icon upright (NoRotate) and the others turn it.
- **The corpus edit test** moves and saves 17 of the pdf.js corpus's 29 `annotation-*.pdf` files; it skips 6 that need a full rewrite (repaired files), 5 holding only form widgets and 1 whose annotations are all locked.
- **Found:** the caret's appearance doesn't turn with the page (#694).

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
