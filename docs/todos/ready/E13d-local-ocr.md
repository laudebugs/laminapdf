# E13d — Client-side OCR on web and local OCR on desktop

| Field | Value |
| --- | --- |
| ID | `E13d` |
| Status | `ready` |
| Plan | [full-editor §E13](../../plans/full-editor.md#e13-ocr-and-conversion) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E13c](./E13c-ocr-text-layer.md) |
| Blocks | None |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e13d-local-ocr` |
| PR title | `feat(ocr): on-device OCR for web and desktop` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Run OCR on the device: a WASM OCR engine in a worker for small documents on web (if ADR 0013's licence and size allow), and a local engine on desktop as an optional download (verified, signed), producing hOCR/ALTO for E13c.

## Context

- ADR 0013

## Scope

**In scope**

- Web OCR worker with page-image rendering via `lamina-render`
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

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
