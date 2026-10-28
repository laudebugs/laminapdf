# E2f — Crop, resize, page boxes and orientation

| Field | Value |
| --- | --- |
| ID | `E2f` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2b](./E2b-page-structure-ops.md), [E2e](./E2e-content-append-api.md) |
| Blocks | [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2f-page-geometry` |
| PR title | `feat(ops): page boxes, crop with warning, resize via content scaling` |
| Size | M (200–600) |
| PR | — |

## Goal

Page geometry ops: crop (sets `/CropBox`, with a user-visible warning that cropped content remains in the file), resize (changes `/MediaBox` and scales content through the append API's `q <scale cm> … Q` wrap, also scaling annotation rects), set any page box (Media, Crop, Bleed, Trim, Art), and portrait ↔ landscape (via `/Rotate` or box swap + transform).

## Context

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)
- [06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina): "crop that only hides content" is a correctness trap that must be tested
- ISO 32000-2 §14.11.2 (page boundaries)

## Scope

**In scope**

- Ops: `SetPageBoxes { pages, boxes }`, `CropPages { pages, rect }`, `ResizePages { pages, size, mode: Scale|Extend|Anchor }`, `SetOrientation { pages, orientation, method }`
- Resize scales annotation `/Rect`, `/QuadPoints`, `/InkList`, `/Vertices` and widget rects consistently
- `OpReport` warning `CroppedContentRemains` for crop; the UI (E2h) offers "remove hidden content" later (E11d/E12)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.09` crop display area; `C.10` resize page dimensions; `C.11` modify margins and page boxes; `C.12` portrait ↔ landscape
- **Crates and packages:** `lamina-ops`, `lamina-content` (append API)
- **`Op` variants:** `SetPageBoxes`, `CropPages`, `ResizePages`, `SetOrientation` (class `PageStructure`; resize also `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: page insertion/deletion/reordering is class `PageStructure`, a violation under DocMDP P1/P2/P3 and of approval signatures' coverage; blocked by default and allowed only by the explicit invalidation override (warning names signatures)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.09` → `round-trip` (core, pdf); `C.10` → `round-trip` (core, pdf); `C.11` → `round-trip` (core, pdf); `C.12` → `round-trip` (core, pdf)

## Tasks

- [ ] Box validation (inside MediaBox, normalised rects) with tolerant reading of odd boxes
- [ ] Resize math with anchor modes and annotation transforms
- [ ] Crop warning in `Applied`; a test proves cropped text is still extractable (the trap is documented by a test)
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Resizing A4 → Letter with `Scale` renders the same content scaled (PDFium diff after scaling) and annotations stay aligned with their text
- [ ] A test asserts text outside a new CropBox is still extracted by pdftotext (the warning is justified) and that the UI receives `CroppedContentRemains`

## Tests

- Geometry unit tests
- Render diffs
- Extraction test for the crop trap

## Docs to update

- API docs: page geometry
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Rotated pages (`/Rotate 90`) plus resize anchors are easy to get wrong; test all four rotations

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
