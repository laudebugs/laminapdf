# E15d — Matterhorn machine checks, missing fonts/Unicode checks and fix-ups

| Field | Value |
| --- | --- |
| ID | `E15d` |
| Status | `ready` |
| Plan | [full-editor §E15](../../plans/full-editor.md#e15-accessibility) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E15b](./E15b-structure-tree-editing.md) |
| Blocks | [E15e](./E15e-pdfua-output.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e15d-accessibility-checks` |
| PR title | `feat(a11y): accessibility checker with fix-ups` |
| Size | L (600–1,500) |
| PR | — |

## Goal

An in-app accessibility checker implementing the machine-checkable Matterhorn conditions (shared with B8 profiles), ToUnicode/missing-font coverage checks, results linked to page locations, and one-click fix-ups where safe (set language, mark untagged content as artifact, add missing alt placeholders flagged for review, fix role maps).

## Context

- [03 §9.4 PDF/UA, WTPDF, Matterhorn](../../research/pdf/03-text-and-fonts.md#94-pdfua-wtpdf-matterhorn)

## Scope

**In scope**

- Checker engine
- Fix-up ops
- Checker panel

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `L.08` check missing fonts / Unicode mappings; `L.09` run accessibility validation; `L.10` fix tagging and structure issues
- **Crates and packages:** `lamina-model` (checks shared with B8), `@lamina/ui`
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Document` (or `Annotation` where noted) → DocMDP violation unless P3 allows it; gated
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `L.08` → `read` (core, pdf, web, desktop); `L.09` → `read` (core, pdf, web, desktop); `L.10` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Checks
- [ ] Fix-ups
- [ ] Panel

## Acceptance criteria

- [ ] Checker results agree with veraPDF on the machine-checkable rules for the fixture set (agreement test)

## Tests

- Agreement tests

## Docs to update

- User help
- [full-editor §E15](../../plans/full-editor.md#e15-accessibility) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
