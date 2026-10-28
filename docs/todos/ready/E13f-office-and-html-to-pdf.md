# E13f — Office → PDF and HTML → PDF (Browser Run) with lamina post-processing

| Field | Value |
| --- | --- |
| ID | `E13f` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E13e](./E13e-server-jobs-infrastructure.md) |
| Blocks | None |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13f-office-and-html-to-pdf` |
| PR title | `feat(convert): Office and HTML to PDF conversion` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Convert Word/Excel/PowerPoint to PDF in the LibreOffice container (server, opt-in; optional local LibreOffice on desktop if installed) and HTML/URLs to PDF via Browser Run, then post-process with lamina (metadata, optional tags/PDF/A where feasible).

## Context

- ADR 0013
- [05 §1.4 HTML/CSS → PDF](../../research/pdf/05-creating-pdfs-from-code.md#14-htmlcss--pdf)
- [05 §1.6 office conversion](../../research/pdf/05-creating-pdfs-from-code.md#16-template--declarative-approaches-and-office-conversion)

## Scope

**In scope**

- Job types `office_to_pdf`, `html_to_pdf`
- Desktop local LibreOffice detection and headless invocation
- Post-processing step

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `H.01` Word / Excel / PowerPoint → PDF; `H.03` HTML / webpages → PDF
- **Crates and packages:** `apps/jobs`, `containers/office`, `apps/desktop`
- **`SaveConstraint`:** no new save behaviour beyond the round's: OCR layer and converted outputs follow normal rules (incremental / new document)
- **Web vs desktop:** web: server only; desktop: local if LibreOffice is installed, else server opt-in
- **Server features:** Containers (LibreOffice), Browser Run
- **Capability-matrix rows to flip:** `H.01` → `write` (web, desktop); `H.03` → `write` (web, desktop)

## Tasks

- [ ] Job types
- [ ] Desktop local path
- [ ] Post-processing
- [ ] Quality fixtures

## Acceptance criteria

- [ ] Fixture documents convert with page counts and text matching references
- [ ] HTML → PDF of a fixture page passes qpdf and has metadata set

## Post-merge checks

None.

## Tests

- Conversion fixtures

## Docs to update

- User help: converting
- [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
