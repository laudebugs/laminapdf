# E6f — Touched-page reload time: measure and budget

| Field | Value |
| --- | --- |
| ID | `E6f` |
| Status | `ready` |
| Plan | [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6b](./E6b-place-images-and-shapes.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `perf/e6f-reload-budget` |
| PR title | `perf(viewer): measure and budget touched-page reload after content edits` |
| Size | M (200–600) |
| PR | — |

## Goal

Measure how long it takes, on web (PDFium-WASM) and desktop (native PDFium), to show a content edit: incremental update → reload in PDFium → re-render touched pages; set budgets, enforce them in the performance harness, and optimise the reload path (reuse loaded document where possible).

## Context

- [architecture §6](../../plans/architecture.md#6-risks) two-parsers risk
- [web C1](../../plans/web-app.md#c1-rendering-spikes): `pdfium-wasm` spike measured reload after incremental save
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) rendering note

## Scope

**In scope**

- Benchmarks for 1, 10, 100 MB files with one edit
- Budget numbers recorded in the E6 feature plan and enforced in CI (C3 harness / desktop bench)
- Optimisations: append-only reload, page-level cache invalidation

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `@lamina/viewer`, `lamina-render`, `apps/desktop`
- **`SaveConstraint`:** no new save behaviour beyond the round's: incremental on `Free`; content changes are DocMDP violations under any certification (gated, override names the signatures); with approval signatures only they are `AppendOnly` incremental updates
- **Web vs desktop:** measured separately per shell
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Bench harness
- [ ] Budgets
- [ ] Optimise
- [ ] CI gate

## Acceptance criteria

- [ ] Budgets met on the reference machine for both shells; CI fails on regressions > 20%

## Post-merge checks

None.

## Tests

- Benchmarks in the perf lane

## Docs to update

- Performance budgets doc
- [full-editor §E6](../../plans/full-editor.md#e6-adding-content-to-pages) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
