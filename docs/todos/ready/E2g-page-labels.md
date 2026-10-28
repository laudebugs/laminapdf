# E2g — Page labels editor

| Field | Value |
| --- | --- |
| ID | `E2g` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2b](./E2b-page-structure-ops.md) |
| Blocks | [E2h](./E2h-page-organiser-ui.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2g-page-labels` |
| PR title | `feat(ops): page label ranges` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Edit the `/PageLabels` number tree (style, prefix, start) per range, keep labels consistent when pages move, and expose a labels panel.

## Context

- [04 §1.5 Page labels](../../research/pdf/04-interactive-security.md#15-page-labels-thumbnails-viewer-preferences-page-modelayout)
- ISO 32000-2 §12.4.2 (page labels)

## Scope

**In scope**

- `SetPageLabels { ranges: Vec<LabelRange> }` op over the `NumberTree` view from B2
- Label maintenance when pages are inserted/deleted/moved (ranges keyed by page index, rebuilt from `PageId`s)
- Labels editor dialog in `@lamina/ui` (ranges table, preview)

**Out of scope** (and which todo covers it)

- Headers/footers and page numbers stamped as content (rest of C.14) → [E6d](E6d-watermarks-headers-footers.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.14` labels, page numbers, headers/footers; `I.10` page labels
- **Crates and packages:** `lamina-model` (`PageLabels` write), `lamina-ops`, `@lamina/ui`
- **`Op` variants:** `SetPageLabels` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental; signed: class `Document` is a DocMDP violation at every level (warn and gate)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `I.10` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Number-tree writer (balanced, `/Nums` leaf limit)
- [ ] Op and maintenance hooks in page ops
- [ ] Dialog and thumbnail captions using labels

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Labels shown in pdf.js and PDFium match the editor's preview on 5 fixtures (roman, alpha, prefixed, restarts)

## Tests

- Unit tests for range rebuilding after page moves
- Round-trip and pdf.js label facts

## Docs to update

- API docs: page labels
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
