# E7j — Edit-corpus tests and E7 round close

| Field | Value |
| --- | --- |
| ID | `E7j` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E7g](./E7g-search-and-replace.md), [E7h](./E7h-vector-clipping-masks.md), [E7i](./E7i-structure-and-conformance-maintenance.md) |
| Blocks | None |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `test/e7j-edit-corpus` |
| PR title | `test(content): edit corpus with region-limited render diffs` |
| Size | M (200–600) |
| PR | — |

## Goal

A corpus of real documents with scripted edits (text change, image replace, path recolour, delete) where the rendered output must be identical except in the edited regions (masks), in PDFium, pdf.js and hayro; plus a list of documents lamina refuses to edit (read-only fallback) with reasons; then verify E7 rows.

## Context

- [full-editor §8](../../plans/full-editor.md#8-risks) risks: "corpus of real edits; never break files we can't edit"

## Scope

**In scope**

- Corpus (licensed, provenance)
- Edit scripts as op lists (ADR 0006 JSON)
- Masked diff harness
- Nightly lane

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** tests
- **`SaveConstraint`:** no new save behaviour beyond the round's: incremental on `Free`; struct tree and MCIDs maintained; PDF/A/UA claims re-checked on save
- **Web vs desktop:** n/a
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** verifies E7 rows

## Tasks

- [ ] Corpus
- [ ] Scripts
- [ ] Harness
- [ ] Capability check

## Acceptance criteria

- [ ] Nightly suite green
- [ ] No file in the corpus is corrupted by an attempted edit (qpdf `--check`, opens in all three renderers)

## Tests

The suite itself.

## Docs to update

- `fixtures/README.md`
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
