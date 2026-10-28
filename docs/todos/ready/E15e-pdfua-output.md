# E15e — PDF/UA output and veraPDF agreement in CI; round close

| Field | Value |
| --- | --- |
| ID | `E15e` |
| Status | `ready` |
| Plan | [full-editor §E15](../../plans/full-editor.md#e15-accessibility) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E15c](./E15c-tagging-tools.md), [E15d](./E15d-accessibility-checks.md) |
| Blocks | [E16e](./E16e-preflight-and-conversion.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e15e-pdfua-output` |
| PR title | `feat(a11y): PDF/UA export with veraPDF agreement tests` |
| Size | M (200–600) |
| PR | — |

## Goal

"Save as PDF/UA": run checks, apply required metadata (`pdfuaid:part`, `/MarkInfo`, `/ViewerPreferences /DisplayDocTitle`), refuse with a report when checks fail; veraPDF runs in CI (external, or as an optional Container per plan — GPL, never linked).

## Context

- [full-editor §E15](../../plans/full-editor.md#e15-accessibility)

## Scope

**In scope**

- Export flow
- CI agreement tests

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `L.11` PDF/UA-compliant output; `H.12` convert to PDF/A, PDF/X, PDF/UA
- **Crates and packages:** `lamina-save`, `lamina-model`
- **`SaveConstraint`:** incremental on `Free`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `L.11` → `round-trip` (core, pdf, web, desktop); `H.12` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Flow
- [ ] CI
- [ ] Capability check

## Acceptance criteria

- [ ] Exports of fixed fixtures pass veraPDF PDF/UA-1

## Post-merge checks

None.

## Tests

- veraPDF CI

## Docs to update

- User help
- [full-editor §E15](../../plans/full-editor.md#e15-accessibility) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
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
