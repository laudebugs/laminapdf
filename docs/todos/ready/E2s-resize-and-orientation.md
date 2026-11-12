# E2s — Resize pages and change their orientation

| Field | Value |
| --- | --- |
| ID | `E2s` |
| Status | `ready` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2e](./E2e-content-append-api.md), [E2f](./E2f-page-geometry.md), [E2r](./E2r-planner-page-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2s-resize-and-orientation` |
| PR title | `feat(ops): resize pages and change orientation` |
| Size | L (400–800): two ops that scale content through [E2e](E2e-content-append-api.md)'s writer and map annotations and destinations through one matrix, with bindings; the box ops are [E2f](E2f-page-geometry.md)'s |
| PR | — |

## Goal

Resize pages (scaling the content, or extending the page around an anchor) and switch them between portrait and landscape, with annotations, destinations and structure boxes following the content. Split out of [E2f](E2f-page-geometry.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos) so each fits one reviewable PR.

## Context

- **Design (Paper):** [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): the payloads
- ISO 32000-2 §14.11.2 (page boundaries), §12.3.2.2 (explicit destinations)

## Scope

**In scope**

- Ops, with the payloads of [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `ResizePages` and `SetOrientation`; each reports its touched widgets to [E2r](E2r-planner-page-checks.md)'s checks
- Resize scales annotation `/Rect`, `/QuadPoints`, `/InkList`, `/Vertices`, `/CL`, `/L` and widget rects consistently
- Resize maps every explicit destination that targets a resized page through the same matrix, wherever it lives: outline items (`/Dest` and `/A /GoTo /D`), link annotations, named destinations (the name tree and the PDF 1.1 `/Dests` dictionary), `/OpenAction`, and `GoTo` actions in other annotations and fields. Per ISO 32000-2 §12.3.2.2: `/XYZ left top` → the transformed point (null coordinates stay null, zoom unchanged); `/FitH top` and `/FitBH top` → the transformed `top`; `/FitV left` and `/FitBV left` → the transformed `left`; `/FitR l b r t` → the transformed, normalised rectangle; `/Fit` and `/FitB` unchanged. Structure-element `/BBox` attributes (Layout attributes, ISO 32000-2 §14.8.5.4) on elements whose `/Pg` is the page are transformed too. Named destinations are rewritten through [E2j](E2j-page-tree-and-tree-writers.md)'s name-tree writer; a destination shared by several pages is touched only for the resized ones
- `SetOrientation { swapBox }`, which changes `/MediaBox` rather than `/Rotate`, applies the same destination and `/BBox` mapping

**Out of scope** (and which todo covers it)

- Setting boxes and cropping → [E2f](E2f-page-geometry.md)
- The page-setup dialogs → [E2g](E2g-page-labels.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.10` resize page dimensions; `C.12` portrait ↔ landscape
- **Crates and packages:** `laminapdf-ops`, `laminapdf-content` (append API), the façade and bindings
- **`Op` variants:** `ResizePages`, `SetOrientation` (each exactly one class, ADR 0006 §1: `PageStructure`; resizing also wraps content and moves annotations, but `PageStructure` is forbidden at every DocMDP level, so it is the most restrictive class and the one reported)
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.10` → `round-trip` (core, pdf); `C.12` → `round-trip` (core, pdf)

## Tasks

- [ ] Resize math with the anchor modes and the annotation transforms
- [ ] One function that maps an explicit destination array through a matrix (every destination kind), applied to outlines, links, named destinations, `/OpenAction` and `GoTo` actions; structure `/BBox` attributes
- [ ] Bindings (`doc.pages.resize()`, `.setOrientation()`) and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict of the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Resizing A4 → Letter with `scale` renders the same content scaled (PDFium diff after scaling), and annotations stay aligned with their text
- [ ] Destinations follow the resize: on a fixture with an outline item `/XYZ 72 700 0`, a link `/FitR`, a named destination `/FitH`, an `/OpenAction /FitV` and a structure element with a Layout `/BBox`, all on page 1, resizing A4 → Letter transforms each exactly (a unit test on the arrays), and following each in pdf.js lands on the same content as before

## Post-merge checks

None.

## Tests

- Geometry unit tests in all four rotations
- Render diffs

## Docs to update

- API docs: page geometry
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Rotated pages (`/Rotate 90`) plus resize anchors are easy to get wrong; test all four rotations

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
