# E14c — Visual and structural compare

| Field | Value |
| --- | --- |
| ID | `E14c` |
| Status | `ready` |
| Plan | [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [C5b](./C5b-find-in-document.md), [E14b](./E14b-revision-browser.md) |
| Blocks | None |
| Dependency notes | [E14b](E14b-revision-browser.md), `C5` ([web-app §C5](../../plans/web-app.md#c5-text-selection-and-search): text layer and search) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e14c-compare` |
| PR title | `feat(compare): visual and structural document compare` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Compare two documents or revisions: page alignment, pixel diff via `lamina-render`, text diff via `lamina-text`, object/annotation diff, with a side-by-side and overlay view and an exportable report.

## Context

- [05 §3.10 Diff and compare](../../research/pdf/05-creating-pdfs-from-code.md#310-diff-and-compare)

## Scope

**In scope**

- Compare engine
- Views
- Report export (PDF/JSON)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.13` compare versions visually and structurally
- **Crates and packages:** `lamina-render`, `lamina-text`, `lamina-model`, `@lamina/ui`
- **`SaveConstraint`:** read-only (report is a new document)
- **Web vs desktop:** identical; web caps pages per `Limits`
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.13` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Engine
- [ ] Views
- [ ] Report

## Acceptance criteria

- [ ] Known differences in fixture pairs are all found; identical pairs report none

## Post-merge checks

None.

## Tests

- Fixture pairs

## Docs to update

- User help
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
