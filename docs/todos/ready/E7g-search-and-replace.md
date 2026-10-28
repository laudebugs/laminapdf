# E7g — Search and replace across the document

| Field | Value |
| --- | --- |
| ID | `E7g` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [C5b](./C5b-find-in-document.md), [E7f](./E7f-font-handling-and-styling.md) |
| Blocks | [E7j](./E7j-edit-corpus.md) |
| Dependency notes | [E7f](E7f-font-handling-and-styling.md), `C5` ([web-app §C5](../../plans/web-app.md#c5-text-selection-and-search): text layer and search) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7g-search-and-replace` |
| PR title | `feat(content): document-wide search and replace` |
| Size | M (200–600) |
| PR | — |

## Goal

Find (B9/C5 search) and replace across pages using the E7e/E7f editing path, with preview, per-match skip, and a report of matches that could not be replaced.

## Context

- [web C5](../../plans/web-app.md#c5-text-selection-and-search)

## Scope

**In scope**

- `ReplaceText { matches, replacement }` composite op in one transaction
- Replace UI in the find bar

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.08` search and replace throughout the document
- **Crates and packages:** `lamina-content`, `lamina-text`, `@lamina/ui`
- **`Op` variants:** `ReplaceText` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level and approval signatures' integrity → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.08` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Op
- [ ] UI
- [ ] Report

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Replace-all on a 100-page fixture completes within budget and undo restores it

## Tests

- Integration and e2e

## Docs to update

- User help
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
