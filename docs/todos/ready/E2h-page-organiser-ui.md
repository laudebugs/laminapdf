# E2h — Page organiser UI: thumbnails drag-and-drop, multi-select and page dialogs

| Field | Value |
| --- | --- |
| ID | `E2h` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1e](./E1e-save-and-constraint-banner.md), [E2c](./E2c-cross-document-pages.md), [E2d](./E2d-merge-and-split.md), [E2f](./E2f-page-geometry.md), [E2g](./E2g-page-labels.md) |
| Blocks | [E2i](./E2i-signed-gating-and-close.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2h-page-organiser-ui` |
| PR title | `feat(ui): page organiser with drag-and-drop thumbnails` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Expose every E2 op in the apps: a page-organiser mode with multi-select thumbnails, drag-and-drop reordering and insertion from files, context menus and dialogs (insert, rotate, crop with warning, resize, boxes, split, extract, merge), keyboard-accessible on both shells.

## Context

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) web vs desktop
- [web C6](../../plans/web-app.md#c6-navigation-print-and-properties): the thumbnail sidebar this grows from
- [desktop D3](../../plans/desktop-app.md#d3-files): drag-drop onto window and icon

## Scope

**In scope**

- Organiser mode (grid of thumbnails) and sidebar operations; multi-select (shift/cmd), drag reorder, drop files to insert
- Dialogs: insert blank/from file, rotate, crop (with "content remains" warning), resize, page boxes, split, extract, merge, replace
- Outputs: web downloads single files or a zip for split/export; desktop writes to a chosen folder
- Desktop: merge from a Finder/Explorer multi-file selection (open-with or drop)
- Keyboard: arrow navigation, Cmd/Ctrl+Shift+arrows to move pages, Delete key; ARIA grid semantics

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.02` insert pages from other PDFs; `C.03` delete and duplicate pages; `C.04` reorder pages; `C.05` extract selected pages; `C.06` merge multiple documents; `C.07` split by count, ranges, bookmarks; `C.08` rotate pages 90/180/270; `C.09` crop display area; `C.10` resize page dimensions; `C.11` modify margins and page boxes; `C.12` portrait ↔ landscape; `C.13` reverse page order; `C.16` replace pages, preserving the rest; `H.10` export pages/ranges
- **Crates and packages:** `@lamina/ui`, `@lamina/viewer` (thumbnail drag layer), `apps/web` (zip via a small streaming zip writer, licence-checked), `apps/desktop`
- **`SaveConstraint`:** banner from E1e shows violations as the user drags on signed files; drops are refused before applying when `would_violate`
- **Web vs desktop:** identical except outputs (zip vs folder) and desktop multi-file merge from the OS
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01` → `round-trip` (web, desktop); `C.02` → `round-trip` (web, desktop); `C.03` → `round-trip` (web, desktop); `C.04` → `round-trip` (web, desktop); `C.06` → `round-trip` (web, desktop); `C.08` → `round-trip` (web, desktop); `C.09` → `round-trip` (web, desktop); `C.10` → `round-trip` (web, desktop); `C.11` → `round-trip` (web, desktop); `C.12` → `round-trip` (web, desktop); `C.13` → `round-trip` (web, desktop); `C.16` → `round-trip` (web, desktop); `C.05` → `write` (web, desktop); `C.07` → `write` (web, desktop); `H.10` → `write` (web, desktop)

## Tasks

- [ ] Organiser mode in the mode state machine
- [ ] Drag-and-drop with auto-scroll and insertion indicator; drop-to-insert from OS files
- [ ] Dialogs and context menus
- [ ] Zip writer for web; folder output on desktop (Tauri dialog plugin)
- [ ] e2e tests tagged with capability IDs

## Acceptance criteria

- [ ] e2e on web and desktop for every op: perform via UI, save, reopen, verify page facts
- [ ] Drag reorder of 200 thumbnails stays responsive (no long task > 50 ms in the C3 harness)
- [ ] Keyboard-only reorder works; axe zero serious violations
- [ ] On a signed fixture, a drag that would violate DocMDP shows the named signature before applying

## Tests

- Playwright `apps/web/e2e/organiser.spec.ts`
- WebdriverIO `apps/desktop/e2e/organiser.e2e.ts`
- Vitest for selection model

## Docs to update

- User help: organising pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Thumbnail rendering cost on huge documents during drag: reuse C3 canvas pool and low-res thumbs

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
