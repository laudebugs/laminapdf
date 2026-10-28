# E14b — Revision index and revision browser

| Field | Value |
| --- | --- |
| ID | `E14b` |
| Status | `ready` |
| Plan | [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E10h](./E10h-revision-diff-and-attacks.md), [E14a](./E14a-compare-revisions-batch-feature-plan.md) |
| Blocks | [E14c](./E14c-compare.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e14b-revision-browser` |
| PR title | `feat(cos): revision index and read-only revision browser` |
| Size | M (200–600) |
| PR | — |

## Goal

Expose the revision index built for signature diffing in E10h (`/Prev` chain, byte ranges, objects changed per revision) as a public API in `lamina` and `@lamina/pdf`, and let users open any prior revision read-only in a revisions panel.

## Context

- [01 §7 Incremental updates](../../research/pdf/01-file-structure.md#7-incremental-updates-756)
- [04 §13](../../research/pdf/04-interactive-security.md#13-implications-for-laminas-design) item 2 revision-aware reader

## Scope

**In scope**

- `doc.revisions()` with offsets and changed objects
- `Document::open_revision(n)` read-only
- Revisions panel

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.12` inspect incremental updates and revisions
- **Crates and packages:** `lamina-cos`, `lamina-save`, `@lamina/ui`
- **`SaveConstraint`:** read-only
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.12` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Index
- [ ] Open revision
- [ ] Panel

## Acceptance criteria

- [ ] Revision counts and offsets match qpdf `--show-xref` style facts on corpus files
- [ ] Prior revisions render in PDFium (truncated byte source)

## Post-merge checks

None.

## Tests

- Corpus facts

## Docs to update

- API docs
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
