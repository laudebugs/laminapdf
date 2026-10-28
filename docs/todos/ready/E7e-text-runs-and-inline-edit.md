# E7e — Text-run detection and in-place text editing with the existing subset

| Field | Value |
| --- | --- |
| ID | `E7e` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E7c](./E7c-content-ir.md) |
| Blocks | [E7f](./E7f-font-handling-and-styling.md), [E7i](./E7i-structure-and-conformance-maintenance.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7e-text-runs-and-inline-edit` |
| PR title | `feat(content): text run detection and in-place text editing` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Detect editable text runs/lines (MCIDs where tagged, heuristics otherwise), edit text in place re-encoding with the same font when the subset has the glyphs, delete/replace text, and fall back to read-only with a clear reason when not possible.

## Context

- [03 §1](../../research/pdf/03-text-and-fonts.md#1-the-core-insight-pdf-stores-positioned-glyphs-not-text) (positioned glyphs, not text)
- [03 §11.2](../../research/pdf/03-text-and-fonts.md#11-implications-for-laminas-design)
- ADR 0022

## Scope

**In scope**

- Run detection API
- Glyph availability check via font program + encoding/ToUnicode inversion
- Re-encode and reposition following glyphs on the same line
- `EditText { run, new_text }` op
- In-place editor UI with best-effort label

**Out of scope** (and which todo covers it)

- New subsets, substitution, font changes → [E7f](E7f-font-handling-and-styling.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.02` edit existing text; `A.03` delete or replace text
- **Crates and packages:** `lamina-content`, `lamina-text`, `@lamina/ui`
- **`Op` variants:** `EditText` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Content` violates every DocMDP level and approval signatures' integrity → blocked unless the user confirms invalidating the named signatures
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.02` → `write` (core, pdf, web, desktop); `A.03` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Detection
- [ ] Glyph check
- [ ] Re-encode
- [ ] Op
- [ ] UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Edited text extracts as typed in pdf.js and pdftotext
- [ ] Runs that cannot be edited are reported with a reason code

## Tests

- Edit corpus
- Extraction tests

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
