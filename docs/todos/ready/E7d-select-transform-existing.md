# E7d — Select, move, resize, rotate, delete and replace existing objects; stacking order

| Field | Value |
| --- | --- |
| ID | `E7d` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6e](./E6e-content-tools-ui.md), [E7c](./E7c-content-ir.md) |
| Blocks | [E7h](./E7h-vector-clipping-masks.md), [E18g](./E18g-leases-and-base-revisions.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7d-select-transform-existing` |
| PR title | `feat(content): transform, delete, replace and reorder existing objects` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Edit existing objects: select (hit-test), move/resize/rotate (`cm` changes), delete (remove ops; drop unused resources), replace images (swap XObject), crop existing images (clip), change stacking order, delete text runs; UI handles reused from E6e.

## Context

- ADR 0022
- [02 §3 Coordinate systems](../../research/pdf/02-imaging-model.md#3-coordinate-systems-and-transformations)
- [06 §3 group B](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms)

## Scope

**In scope**

- `EditContent { page, edits: Vec<ContentEdit> }` op with ContentEdit = Transform | Delete | ReplaceImage | Reorder | Clip
- Copy-on-write for shared streams/XObjects
- UI: selection of existing objects in `contentEdit` mode

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.07` move/resize/rotate text blocks; `B.01` insert, delete, replace images; `B.02` resize, rotate, crop, reposition; `B.08` change stacking order; `B.12` transformations on graphical objects; `A.03` delete or replace text
- **Crates and packages:** `lamina-content`, `lamina-ops`, `@lamina/ui`
- **`Op` variants:** `EditContent` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked unless the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock` (the signature panel, E10h, reports the later content change)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.07` → `round-trip` (core, pdf, web, desktop); `B.01` → `round-trip` (core, pdf, web, desktop); `B.02` → `round-trip` (core, pdf, web, desktop); `B.08` → `round-trip` (core, pdf, web, desktop); `B.12` → `round-trip` (core, pdf, web, desktop); `A.03` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Op
- [ ] Copy-on-write
- [ ] UI
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Edited pages differ from originals only inside the edited regions (render diff mask)

## Post-merge checks

None.

## Tests

- Edit-corpus subset
- Render diffs
- e2e

## Docs to update

- User help: editing objects (best-effort label)
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
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
