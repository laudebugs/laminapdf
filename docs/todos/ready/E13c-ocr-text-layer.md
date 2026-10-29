# E13c — Invisible OCR text layer from hOCR/ALTO

| Field | Value |
| --- | --- |
| ID | `E13c` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B4b](../ongoing/B4b-shaping-bidi-line-breaking.md), [B4c](../done/B4c-standard-14-and-simple-fonts.md), [E2e](./E2e-content-append-api.md), [E13b](./E13b-ocr-and-conversion-feature-plan.md) |
| Blocks | [E13d](./E13d-local-ocr.md), [E13e](./E13e-server-jobs-infrastructure.md) |
| Dependency notes | [E13b](E13b-ocr-and-conversion-feature-plan.md), [E2e](E2e-content-append-api.md), `B4` ([pdf-library §B4](../../plans/pdf-library.md#b4-fonts-and-text): fonts, shaping, subsetting) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13c-ocr-text-layer` |
| PR title | `feat(build): invisible OCR text layer from hOCR and ALTO` |
| Size | M (200–600) |
| PR | — |

## Goal

Turn OCR output (hOCR or ALTO: words, boxes, baselines, confidence) into an invisible text layer (render mode 3) over scanned pages using a glyphless/fallback font with correct ToUnicode, appended via the append API, optionally tagged.

## Context

- [03 §8.3 Scanned PDFs and OCR](../../research/pdf/03-text-and-fonts.md#83-scanned-pdfs-and-ocr)
- OCRmyPDF's text-layer approach (reference for the technique only; ocrmypdf is not used because it needs Ghostscript, AGPL; see ADR 0013)

## Scope

**In scope**

- hOCR and ALTO parsers (safe XML)
- Text layer builder with per-word horizontal scaling to fit boxes
- `AddOcrLayer { page, ocr: BlobId }` op

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.11` convert outlined or scanned text via OCR; `H.13` scanned → searchable PDF
- **Crates and packages:** `lamina-build`, `lamina-text`, `lamina-content`
- **`Op` variants:** `AddOcrLayer` (class `Content`)
- **`SaveConstraint`:** `Free` → incremental; signed: the invisible text layer is a `Content` change, a DocMDP violation at every certification level (gated, override names the signatures); with approval signatures only `AppendOnly`
- **Web vs desktop:** identical (engines differ, E13d)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `A.11` → `write` (core, pdf); `H.13` → `write` (core, pdf)

## Tasks

- [ ] Parsers
- [ ] Builder
- [ ] Op

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Selecting text over a scan selects the OCR words at the right positions in pdf.js and PDFium
- [ ] pdftotext extracts the OCR text

## Post-merge checks

None.

## Tests

- hOCR/ALTO fixtures
- Extraction tests

## Docs to update

- API docs: OCR layer
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
