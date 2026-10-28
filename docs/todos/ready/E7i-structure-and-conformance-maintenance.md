# E7i — Struct-tree and MCID maintenance; PDF/A and PDF/UA re-checks on save

| Field | Value |
| --- | --- |
| ID | `E7i` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [B8b](./B8b-conformance-profiles-and-verapdf.md), [E7e](./E7e-text-runs-and-inline-edit.md) |
| Blocks | [E7j](./E7j-edit-corpus.md), [E15b](./E15b-structure-tree-editing.md) |
| Dependency notes | [E7e](E7e-text-runs-and-inline-edit.md), `B8` ([pdf-library §B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa): tagged PDF, PDF/UA, PDF/A) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7i-structure-and-conformance-maintenance` |
| PR title | `feat(content): keep tagging valid across content edits` |
| Size | M (200–600) |
| PR | — |

## Goal

Keep tagged documents valid across E6/E7 edits: update MCIDs, ParentTree, `/ActualText` and `/Alt` when runs change or are deleted; re-check PDF/A and PDF/UA claims on save and warn.

## Context

- [03 §9 Tagged PDF](../../research/pdf/03-text-and-fonts.md#9-tagged-pdf--logical-structure-147149)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa) profiles

## Scope

**In scope**

- MCID/ParentTree maintenance hooks in `EditContent`/`EditText`
- Conformance checks reuse B8 profiles
- Warnings in the save banner

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `lamina-model` (struct tree), `lamina-content`
- **`SaveConstraint`:** as E7
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none new; protects E7 rows' `round-trip` on tagged fixtures

## Tasks

- [ ] Hooks
- [ ] Checks
- [ ] Tests

## Acceptance criteria

- [ ] veraPDF structure checks pass after edits on PDF/UA fixtures
- [ ] Deleting a tagged run removes its struct element or MCR

## Tests

- veraPDF nightly
- Unit

## Docs to update

- Guide: editing tagged PDFs
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
