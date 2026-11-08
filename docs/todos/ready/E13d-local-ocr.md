# E13d — Client-side OCR on web and local OCR on desktop

| Field | Value |
| --- | --- |
| ID | `E13d` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E13c](./E13c-ocr-text-layer.md) |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13d-local-ocr` |
| PR title | `feat(ocr): on-device OCR for web and desktop` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Run OCR on the device: a WASM OCR engine in a worker for small documents on web (if ADR 0013's licence and size allow), and a local engine on desktop as an optional download (verified, signed), producing hOCR/ALTO for E13c.

## Context

- **Design (Paper):** [X01 Make searchable (OCR)](../../design/paper/convert-compare-produce.md#x01-make-searchable-ocr). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0013
- [L3c](../done/L3c-model-plugins.md)'s `OcrEngine` trait (`laminapdf-extract`): this todo's engines implement it, so extraction reads scanned pages with them too; a browser or desktop engine answers the extraction job's OCR requests (`request`/`provide`) when it runs asynchronously.

## Scope

**In scope**

- Web OCR worker with page-image rendering via `laminapdf-render`
- Desktop engine download/verify/run as a sidecar process
- OCR dialog (languages, pages)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.11` convert outlined or scanned text via OCR; `H.13` scanned → searchable PDF
- **Crates and packages:** `apps/web` worker, `apps/desktop/src-tauri` (sidecar)
- **`SaveConstraint`:** no new save behaviour beyond the round's: OCR layer and converted outputs follow normal rules (incremental / new document)
- **Web vs desktop:** web: small docs only, enforced by `Limits`; desktop: full documents, optional engine download
- **Server features:** none (on-device)
- **Capability-matrix rows to flip:** `A.11` → `round-trip` (web, desktop); `H.13` → `round-trip` (web, desktop)

## Tasks

- [ ] Web worker
- [ ] Desktop sidecar (load `/tauri-v2`)
- [ ] Dialog
- [ ] Accuracy fixtures

## Acceptance criteria

- [ ] OCR of fixture scans reaches the character accuracy set in E13b
- [ ] No network request during web OCR except fetching the engine/model assets from our origin

## Post-merge checks

None.

## Tests

- Accuracy fixtures
- e2e

## Docs to update

- User help: OCR
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
