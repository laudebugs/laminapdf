# E2f — Crop, resize, page boxes and orientation

| Field | Value |
| --- | --- |
| ID | `E2f` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2b](./E2b-page-structure-ops.md), [E2e](./E2e-content-append-api.md) |
| Blocks | [E2h](./E2h-page-organiser-ui.md), [E16d](./E16d-colour.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2f-page-geometry` |
| PR title | `feat(ops): page boxes, crop with warning, resize via content scaling` |
| Size | M (200–600) |
| PR | — |

## Goal

Page geometry ops: crop (sets `/CropBox`, with a user-visible warning that cropped content remains in the file), resize (changes `/MediaBox` and scales content through the append API's `q <scale cm> … Q` wrap, also scaling annotation rects), set any page box (Media, Crop, Bleed, Trim, Art), and portrait ↔ landscape (via `/Rotate` or box swap + transform).

## Context

- **Design (Paper):** [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)
- [06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina): "crop that only hides content" is a correctness trap that must be tested
- ISO 32000-2 §14.11.2 (page boundaries)

## Scope

**In scope**

- Ops: `SetPageBoxes { pages, boxes }`, `CropPages { pages, rect }`, `ResizePages { pages, size, mode: Scale|Extend|Anchor }`, `SetOrientation { pages, orientation, method }`
- Resize scales annotation `/Rect`, `/QuadPoints`, `/InkList`, `/Vertices`, `/CL`, `/L` and widget rects consistently
- Resize also maps every **explicit destination** that targets a resized page through the same matrix, wherever it lives: outline items (`/Dest` and `/A /GoTo /D`), link annotations, named destinations (`/Names /Dests` name tree and the PDF 1.1 `/Dests` dictionary), `/OpenAction`, and `GoTo` actions in other annotations/fields. Per ISO 32000-2 §12.3.2.2 (explicit destinations; Table 151 "Destination syntax" in ISO 32000-1): `/XYZ left top` → transformed point (null coordinates stay null, zoom unchanged); `/FitH top` and `/FitBH top` → transformed `top`; `/FitV left` and `/FitBV left` → transformed `left`; `/FitR l b r t` → transformed and normalised rectangle; `/Fit` and `/FitB` unchanged. Structure-element `/BBox` attributes (Layout attributes, ISO 32000-2 §14.8.5.4) on elements whose `/Pg` is the page are transformed too. Named destinations are rewritten through E2b's name-tree writer; destinations shared by several pages are only touched for the resized ones
- `SetOrientation` with a method that changes `/MediaBox` rather than `/Rotate` applies the same destination and `/BBox` mapping
- `OpReport` warning `CroppedContentRemains` for crop; the UI (E2h) offers "remove hidden content" later (E11d/E12)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.09` crop display area; `C.10` resize page dimensions; `C.11` modify margins and page boxes; `C.12` portrait ↔ landscape
- **Crates and packages:** `lamina-ops`, `lamina-content` (append API)
- **`Op` variants:** `SetPageBoxes`, `CropPages`, `ResizePages`, `SetOrientation` (each exactly one class, ADR 0006 §1: `PageStructure`; `ResizePages` also wraps content and moves annotations, but `PageStructure` is already forbidden at every DocMDP level, so it is the most restrictive class and the single one reported)
- **`SaveConstraint`:** `Free` → incremental. Signed: setting page boxes, cropping and orientation change page-dictionary keys (`/MediaBox`, `/CropBox`, `/BleedBox`, `/TrimBox`, `/ArtBox`, `/Rotate`) and are class `PageStructure`; `ResizePages` also changes content (the scaling wrap and the scaled annotation rects) but reports only `PageStructure`, which is equally forbidden. Any of these ops on a page holding a signed signature field's widget is E2b's `Violation::SignedWidgetPage` (the signed appearance would move or scale). DocMDP allows none of these (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations), so under a certification signature each is a violation → blocked by default, allowed only by the override that names the signatures (ADR 0007 §3). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact while changing what the signed page shows, which the signature panel (E10h) reports; resizing or rotating a page that carries a widget of a field locked by FieldMDP/`/Lock` changes that widget and is a violation
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.09` → `round-trip` (core, pdf); `C.10` → `round-trip` (core, pdf); `C.11` → `round-trip` (core, pdf); `C.12` → `round-trip` (core, pdf)

## Tasks

- [ ] Box validation (inside MediaBox, normalised rects) with tolerant reading of odd boxes
- [ ] Resize math with anchor modes and annotation transforms
- [ ] Destination transform: one function mapping an explicit destination array through a matrix (every destination kind), applied to outlines, links, named destinations, `/OpenAction` and `GoTo` actions; struct `/BBox` attributes
- [ ] Crop warning in `Applied`; a test proves cropped text is still extractable (the trap is documented by a test)
- [ ] Bindings and examples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Resizing A4 → Letter with `Scale` renders the same content scaled (PDFium diff after scaling) and annotations stay aligned with their text
- [ ] Destinations follow the resize: on a fixture with an outline item `/XYZ 72 700 0`, a link `/FitR`, a named destination `/FitH`, an `/OpenAction /FitV` and a struct element with a Layout `/BBox`, all on page 1, `ResizePages { Scale }` A4 → Letter transforms each exactly (unit test on the arrays), and navigating each in pdf.js lands on the same content position as before the resize (render-position check)
- [ ] A test asserts text outside a new CropBox is still extracted by pdftotext (the warning is justified) and that the UI receives `CroppedContentRemains`

## Post-merge checks

None.

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
