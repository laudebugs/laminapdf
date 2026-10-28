# E4d — Appearances and creation: ink, shapes, lines and arrows

| Field | Value |
| --- | --- |
| ID | `E4d` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4b](./E4b-annotation-model-and-overlay.md) |
| Blocks | [E4f](./E4f-annotation-tools-ui.md), [E17e](./E17e-measurement.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4d-appearances-ink-and-shapes` |
| PR title | `feat(build): appearance streams for ink and geometric annotations` |
| Size | M (200–600) |
| PR | — |

## Goal

Create Ink, Square, Circle, Line (with `/LE` line endings, leader lines), Polygon and PolyLine annotations with border styles (solid, dashed, cloudy `/BE`), interior colour and opacity, each with generated appearances.

## Context

- [04 §2.2 The annotation zoo](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo)
- [04 §2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter): lamina always writes `/AP` for what it creates ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 3)
- [02 §5 Paths](../../research/pdf/02-imaging-model.md#5-paths): Béziers for circles (κ ≈ 0.5523)

## Scope

**In scope**

- Builders for Ink (smoothed per E4a), Square, Circle, Line (all `/LE` styles), Polygon, PolyLine; cloudy borders
- Rect recomputation including stroke width and line endings

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.04` freehand ink; `D.05` shapes, lines, arrows; `B.06` arrows, paths, freehand drawing
- **Crates and packages:** `lamina-build`
- **`Op` variants:** `AddAnnot` specs `Ink`, `Square`, `Circle`, `Line`, `Polygon`, `PolyLine`
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Annotation` is allowed only under DocMDP P3 or when there is no certification signature (approval signatures alone allow annotations); otherwise a violation → warn and gate. Note the Evil Annotation Attack: annotations added after certification are shown as such in the signature panel (E10)
- **Web vs desktop:** identical; pressure handled in E4f
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.04` → `round-trip` (core, pdf); `D.05` → `round-trip` (core, pdf); `B.06` → `round-trip` (core, pdf)

## Tasks

- [ ] Builders and goldens
- [ ] Line-ending geometry
- [ ] Cloudy border algorithm
- [ ] Bindings

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Render parity pdf.js/PDFium for every `/LE` style and border style

## Post-merge checks

None.

## Tests

- Goldens and render diffs

## Docs to update

- API docs: drawing annotations
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
