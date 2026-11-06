# E16f — Imposition, booklet and N-up

| Field | Value |
| --- | --- |
| ID | `E16f` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E2c](./E2c-cross-document-pages.md), [E16a](./E16a-optimisation-print-feature-plan.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16f-imposition` |
| PR title | `feat(ops): imposition, booklet and N-up layouts` |
| Size | M (200–600) |
| PR | — |

## Goal

Place source pages as Form XObjects on new sheets: N-up grids, saddle-stitch booklets (page order, creep), with marks optional; output a new document.

## Context

- **Design (Paper):** [X10 Print production](../../design/paper/convert-compare-produce.md#x10-print-production). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.2 Page imposition and n-up](../../research/pdf/05-creating-pdfs-from-code.md#32-page-imposition-and-n-up)

## Scope

**In scope**

- `impose(doc, layout) -> Document`
- Dialog with preview

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.13` imposition and booklet; `C.15` booklet / N-up layouts
- **Crates and packages:** `laminapdf-build`
- **`SaveConstraint`:** new document
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `K.13` → `write` (core, pdf, web, desktop); `C.15` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Layouts
- [ ] Dialog

## Acceptance criteria

- [ ] Booklet page order matches reference for 4–64 pages
- [ ] Rendered sheets match goldens

## Post-merge checks

None.

## Tests

- Goldens

## Docs to update

- User help
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
