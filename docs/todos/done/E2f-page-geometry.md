# E2f — Crop pages and set their boxes

| Field | Value |
| --- | --- |
| ID | `E2f` |
| Status | `done` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2r](./E2r-planner-page-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2f-page-geometry` |
| PR title | `feat(ops): page boxes, and crop with its warning` |
| Size | M (200–400): two ops that write page-dictionary keys, box validation and the crop warning, with bindings; resize and orientation, which scale content, are [E2s](../done/E2s-resize-and-orientation.md)'s |
| PR | [#576](https://github.com/laudebugs/laminapdf/pull/576) |

## Goal

Set any page box (Media, Crop, Bleed, Trim, Art) and crop pages by margins, with a warning that content outside the crop stays in the file. Resizing and orientation, which split off to [E2s](../done/E2s-resize-and-orientation.md) in the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos), build on these.

## Context

- **Design (Paper):** [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) (the payloads) and [§3](../../plans/editor/e2-page-organisation.md#3-conformance-the-edits-must-keep) (cropping hides, it doesn't remove)
- [06 §10](../../research/pdf/06-feature-inventory.md#10-implications-for-lamina): "crop that only hides content" is a correctness trap that must be tested
- ISO 32000-2 §14.11.2 (page boundaries)

## Scope

**In scope**

- Ops, with the payloads of [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `SetPageBoxes` and `CropPages`; each reports its touched widgets to [E2r](../done/E2r-planner-page-checks.md)'s checks
- `Applied`/`Edited` warning `CroppedContentRemains` for `CropPages`, and for `SetPageBoxes` when the crop shrinks; the page-setup dialog ([E2g](../ready/E2g-page-labels.md)) shows it, and "remove hidden content" comes later (E11d, E12)

**Out of scope** (and which todo covers it)

- Resize and orientation → [E2s](../done/E2s-resize-and-orientation.md)
- The dialogs → [E2g](../ready/E2g-page-labels.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.09` crop display area; `C.11` modify margins and page boxes
- **Crates and packages:** `laminapdf-ops`, the façade and bindings
- **`Op` variants:** `SetPageBoxes`, `CropPages` (class `PageStructure`)
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint, including the recommended rule for a page that holds a signed widget, are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint)
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.09` → `round-trip` (core, pdf); `C.11` → `round-trip` (core, pdf)

## Tasks

- [x] Box validation (inside the MediaBox, normalised rectangles) with tolerant reading of odd boxes
- [x] `CropPages`' margins mapped from page space to each page's user space, in all four rotations
- [x] The crop warning in `Applied`; a test proves cropped text is still extractable
- [x] Bindings (`doc.setPageBoxes()`, `doc.cropPages()`: methods on the document, as the other page ops are) and examples

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict of the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] One set of margins crops pages of mixed sizes and rotations by the same visible amount (render check in PDFium)
- [x] A test asserts text outside a new CropBox is still extracted by pdftotext (the warning is justified) and that `Edited` carries `CroppedContentRemains`

## As built

- `laminapdf-ops`' `pages/boxes.rs`: `SetPageBoxes { pages, boxes: PageBoxes }` (each of `media`, `crop`, `bleed`, `trim`, `art` absent, `null` or `[x0, y0, x1, y1]`; `media: null` fails to parse) and `CropPages { pages, margins: CropMargins }`. Both are `PageStructure` ops that reshow the widgets on their pages (`Op::page_changes` names them `Reshown`, so a signed widget's page is a `SignedWidget` conflict and a locked field's page breaks its lock, as rotating does). Refusals are `invalid-op`: a box without an area, not finite or outside the media box (the new one when the op sets it), negative or non-finite margins, and margins that leave no page.
- `CropPages` maps the displayed rectangle's corners through `PageFrame::to_user_space`, so rotation and `/UserUnit` are applied; the `boxes.pdf` test checks each rotation's new frame and top-left corner.
- The warning is a list, `Applied::cropped_content_remains` (façade `Edited`, WASM `croppedContentRemains`): the pages whose new effective crop box no longer contains the old one. It is reported for `SetPageBoxes` too, when the crop shrinks, and when a new media box cuts the crop.
- Reading stays tolerant: the current boxes come from the model's readers, which drop a malformed box with a warning.
- Tests: `tests/boxes.rs` (geometry in four rotations, refusals, undo, widgets), `pages_wire.rs` (the frozen `set-page-boxes` and `crop-pages` samples), `pages_props.rs` (crop in the random sequences), `laminapdf-save`'s `page_ops.rs` (verdicts on the signed fixtures) and the round-trip suite (`setPageBoxes` and `cropPages` cases, C.09 and C.11). `crates/laminapdf/tests/crop.rs` shows hidden text still extracted. By hand, pdftotext 24.02 read "Page 1" to "Page 4" from a file cropped to a strip that hides them, which PDFium rendered blank.
- The `crop_pages` example (Rust and TypeScript, byte-identical) over `fixtures/pages/boxes.pdf`, with its golden, render goldens (hayro and PDFium agree) and docs previews; the recipe "Crop pages".

## Post-merge checks

None.

## Tests

- Geometry unit tests in all four rotations
- Render checks
- Extraction test for the crop trap

## Docs to update

- API docs: page geometry
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- A box outside the MediaBox: viewers clip it to the MediaBox (§14.11.2); the op refuses one, while reading tolerates it

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
