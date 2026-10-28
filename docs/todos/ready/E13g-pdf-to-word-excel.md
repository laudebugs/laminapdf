# E13g — PDF → Word and PDF tables → Excel (server)

| Field | Value |
| --- | --- |
| ID | `E13g` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E13e](./E13e-server-jobs-infrastructure.md) |
| Blocks | None |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13g-pdf-to-word-excel` |
| PR title | `feat(convert): PDF to Word and tables to Excel` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Opt-in server conversions using the engines chosen in ADR 0013 (layout analysis for paragraphs/styles; table detection for spreadsheets), with quality fixtures and metrics.

## Context

- ADR 0013
- [03 §8.4 Table extraction](../../research/pdf/03-text-and-fonts.md#84-table-extraction)
- [03 §8.5 Layout analysis](../../research/pdf/03-text-and-fonts.md#85-layout-analysis-and-ml-document-understanding-state-of-the-art-2026)

## Scope

**In scope**

- Job types `pdf_to_docx`, `tables_to_xlsx`
- Quality metrics and fixtures

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.04` PDF → Word; `H.05` PDF tables → Excel
- **Crates and packages:** `apps/jobs`, container image(s)
- **`SaveConstraint`:** no new save behaviour beyond the round's: OCR layer and converted outputs follow normal rules (incremental / new document)
- **Web vs desktop:** web and desktop: server opt-in
- **Server features:** Containers
- **Capability-matrix rows to flip:** `H.04` → `write` (web, desktop); `H.05` → `write` (web, desktop)

## Tasks

- [ ] Job types
- [ ] Fixtures and metrics

## Acceptance criteria

- [ ] Metrics meet the thresholds set in E13b on the fixture set

## Post-merge checks

None.

## Tests

- Quality fixtures

## Docs to update

- User help
- [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
