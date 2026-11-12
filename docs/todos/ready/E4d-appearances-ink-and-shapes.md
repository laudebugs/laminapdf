# E4d — Appearances and creation: ink, shapes, lines and arrows

| Field | Value |
| --- | --- |
| ID | `E4d` |
| Status | `ready` |
| Plan | [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4b](../ongoing/E4b-annotation-model-and-overlay.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e4d-appearances-ink-and-shapes` |
| PR title | `feat(build): ink and drawing annotations with their appearances` |
| Size | L (400–800): four specs, every line ending, the cloudy border and pressure widths |
| PR | — |

## Goal

Ink, Square, Circle, Line (every `/LE` ending, leader lines), Polygon and PolyLine annotations with solid, dashed and cloudy borders, interior colour and opacity, each with a generated appearance.

## Context

- **Design (Paper):** [C02 Ink and shapes](../../design/paper/comment-review.md#c02-ink-and-shapes). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E4 plan §2.1](../../plans/editor/e4-markup-and-drawing-annotations.md#21-ops) and [§11](../../plans/editor/e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) (ink points, smoothing and pressure)
- [04 §2.2 The annotation zoo](../../research/pdf/04-interactive-security.md#22-the-annotation-zoo) and [§2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter)
- [02 §5 Paths](../../research/pdf/02-imaging-model.md#5-paths): Béziers for circles (κ ≈ 0.5523)

## Scope

**In scope**

- Specs `Ink`, `Shape`, `Line` and `Poly`, their patches, and the generators
- Ink: points thinned to 0.25 pt in `/InkList`, Bézier smoothing in the appearance, per-point widths under a private key named by ISO 32000-2 Annex E's rules, drawn as a varying outline
- Every `/LE` style, leader lines, cloudy borders (`/BE`), and `/Rect` grown for stroke width, endings and clouds
- `UpdateAnnot` patches that add or remove ink strokes (the eraser and joined strokes of [E4k](E4k-touch-and-stylus-annotating.md))

**Out of scope** (and which todo covers it)

- The tools → [E4f](E4f-annotation-tools-ui.md); pressure capture → [E4k](E4k-touch-and-stylus-annotating.md)
- Measurement lines (`/Measure`) → not planned in this round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.04` freehand ink; `D.05` shapes, lines, arrows; `B.06` arrows, paths, freehand drawing
- **Crates and packages:** `laminapdf-build`, `laminapdf-ops`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `AddAnnot` with `Ink`, `Shape`, `Line`, `Poly`; their `UpdateAnnot` patches
- **`SaveConstraint`:** as [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `D.04`, `D.05`, `B.06` → `round-trip` (core, pdf)

## Tasks

- [ ] Generators and goldens
- [ ] Line-ending geometry
- [ ] Cloudy border
- [ ] Ink thinning, smoothing and widths
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new spec is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every spec's op reports `Annotation` and `requires_rewrite` false; `planner.would_violate(&op)` on the signed fixtures gives the verdicts of [E4 plan §6](../../plans/editor/e4-markup-and-drawing-annotations.md#6-saving-and-the-save-constraint)
- [ ] Round-trip: open → op → save → reopen in Lamina PDF, pdf.js and PDFium preserves the change and everything else; `qpdf --check` is clean
- [ ] Every `/LE` style and border style renders in pdf.js and PDFium within E4g's threshold
- [ ] A pressure-varying stroke keeps its widths through a colour change

## Post-merge checks

None.

## Tests

- Goldens in `fixtures/golden/annots/drawing.pdf` and render diffs
- The round-trip suite

## Docs to update

- API docs: drawing annotations
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- The private key for pressure widths needs a name that Annex E allows; record the choice in the API docs

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
