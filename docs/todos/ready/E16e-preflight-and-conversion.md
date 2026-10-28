# E16e — Preflight rules as data; PDF/A and PDF/X conversion and validation; transparency flattening

| Field | Value |
| --- | --- |
| ID | `E16e` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E15e](./E15e-pdfua-output.md), [E16c](./E16c-images-and-fonts.md), [E16d](./E16d-colour.md) |
| Blocks | [E16g](./E16g-server-batch-optimise.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16e-preflight-and-conversion` |
| PR title | `feat(preflight): rule-based preflight and PDF/A/X conversion` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Preflight with rules as data (fonts embedded, colour spaces, resolution, overprint, transparency, boxes), validation profiles for PDF/A (shared with B8) and PDF/X, conversion to PDF/A and PDF/X (embed fonts, output intent, XMP identification, remove forbidden features), and transparency flattening per E16a.

## Context

- [05 §3.7](../../research/pdf/05-creating-pdfs-from-code.md#37-conversion-to-pdfa)
- [library B8](../../plans/pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa)

## Scope

**In scope**

- Rule engine and rule files
- Conversion pipelines
- Flattening (or raster fallback with warning)
- Preflight panel

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `K.07` flatten transparency; `K.11` print-ready PDFs / preflight; `K.12` validate PDF/A or PDF/X; `H.12` convert to PDF/A, PDF/X, PDF/UA
- **Crates and packages:** `lamina-model` (profiles), `lamina-save`
- **`SaveConstraint`:** `RequiresRewrite` for optimisation (full rewrite); signed files → `Conflict`, warned and overridden only explicitly
- **Web vs desktop:** identical; desktop recommended for large print files
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `K.07` → `write` (core, pdf, web, desktop); `K.11` → `read` (core, pdf, web, desktop); `K.12` → `read` (core, pdf, web, desktop); `H.12` `write` → `round-trip` (core, pdf, web, desktop) (from the status [E15e](E15e-pdfua-output.md) set; E16e depends on E15e so the flips can't land out of order)

## Tasks

- [ ] Rules
- [ ] Pipelines
- [ ] Flattening
- [ ] Panel

## Acceptance criteria

- [ ] Converted fixtures pass veraPDF PDF/A-2b
- [ ] PDF/X outputs pass the chosen PDF/X rule set

## Post-merge checks

None.

## Tests

- veraPDF
- Rule tests

## Docs to update

- User help: preflight
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
