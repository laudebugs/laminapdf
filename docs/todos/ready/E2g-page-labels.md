# E2g — Page labels and the page-setup dialogs

| Field | Value |
| --- | --- |
| ID | `E2g` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2d](./E2d-merge-and-split.md), [E2f](./E2f-page-geometry.md), [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2g-page-labels` |
| PR title | `feat(ops,ui): page label ranges and page-setup dialogs` |
| Size | M (200–600) |
| PR | — |

## Goal

Edit the `/PageLabels` number tree (style, prefix, start) per range, keep labels consistent when pages move, and give Organise's Page setup ▾ its dialogs: page labels (O10) and, since the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos) moved them here from E2h, crop, resize, page boxes and orientation (O09) over [E2f](E2f-page-geometry.md)'s ops.

## Context

- **Design (Paper):** [O09 Page geometry](../../design/paper/pages-navigation-extraction.md#o09-page-geometry), [O10 Page labels editor](../../design/paper/pages-navigation-extraction.md#o10-page-labels-editor). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §1.5 Page labels](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout)
- ISO 32000-2 §12.4.2 (page labels)

## Scope

**In scope**

- `SetPageLabels { ranges: Vec<LabelRange> }` op over the `NumberTree` view from B2, written with [E2j](E2j-page-tree-and-tree-writers.md)'s number-tree writer (E2g does not write its own)
- Label maintenance is E2j's `PageLabels::remap`, already called by every page op (E2b's insert/delete/move/reverse, E2k's duplicate, E2c's `InsertPagesFrom`/`ReplacePages`/extract, E2d's merge/split). This todo owns the cross-op test that a `SetPageLabels` edit followed by each of those ops keeps every surviving page's label
- Labels editor dialog in `@laminapdf/ui` (ranges table, preview), and label captions on Organise's thumbnails
- The O09 dialogs: Crop… (margins, with the "content remains" warning from `CroppedContentRemains`), Resize…, Page boxes…, Make landscape / Make portrait (`pages.crop`, `pages.resize`, `pages.edit-boxes`, `pages.make-landscape`, `pages.make-portrait`, `pages.edit-labels`)

**Out of scope** (and which todo covers it)

- Headers/footers and page numbers stamped as content (rest of C.14) → [E6d](E6d-watermarks-headers-footers.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.14` labels, page numbers, headers/footers; `I.10` page labels
- **Crates and packages:** `laminapdf-ops`, the façade and bindings, `@laminapdf/ui`
- **`Op` variants:** `SetPageLabels` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental; signed: page labels are class `Document`, a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → warn and gate (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.10` → `round-trip` (core, pdf, web); `C.09`, `C.10`, `C.11`, `C.12` → `round-trip` (web). The desktop column is [E2n](E2n-desktop-organise.md)'s. `C.14` is E6d's ([E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions))

## Tasks

- [ ] `SetPageLabels` op over E2b's number-tree writer
- [ ] Cross-op label test covering `InsertBlankPages`, `DeletePages`, `DuplicatePages`, `MovePages`, `ReversePages`, `InsertPagesFrom`, `ReplacePages`, extract and merge
- [ ] Dialog and thumbnail captions using labels
- [ ] The O09 dialogs and their commands
- [ ] Playwright tests tagged with capability IDs

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] After `SetPageLabels` (roman front matter + arabic body + prefixed appendix), each of `InsertPagesFrom`, `ReplacePages`, `merge` and `extract_pages` gives labels in pdf.js and PDFium that match `PageLabels::remap`'s expectation (no page relabelled except new ones)
- [ ] Labels shown in pdf.js and PDFium match the editor's preview on 5 fixtures (roman, alpha, prefixed, restarts)
- [ ] Playwright in Chromium, Firefox and WebKit, as a user: crop, resize, set boxes, change orientation and edit labels through the dialogs, save, reopen and check the page facts; the crop dialog shows that cropped content remains; axe reports no serious violations

## Post-merge checks

None.

## Tests

- Unit tests for range rebuilding after page moves
- Round-trip and pdf.js label facts
- Playwright `apps/web/e2e/page-setup.test.ts`

## Docs to update

- API docs: page labels
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
