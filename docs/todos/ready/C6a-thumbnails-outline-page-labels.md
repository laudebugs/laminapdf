# C6a — Thumbnails sidebar, outline sidebar, page labels and go-to-page

| Field | Value |
| --- | --- |
| ID | `C6a` |
| Status | `ready` |
| Plan | [web reader §6 C6. Navigation, print and properties](../../plans/web-app.md#c6-navigation-print-and-properties) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B2c](../ongoing/B2c-lamina-model-corpus-fuzz.md), [B6b](./B6b-lamina-pdf-package.md), [C3c](./C3c-viewer-perf-harness.md), [C4c](./C4c-remote-files-encryption-privacy.md), [U1c](./U1c-inverted-l-shell-and-modes.md) |
| Blocks | [C6b](./C6b-links-and-history.md), [C7a](./C7a-accessibility-and-theming.md) |
| Dependency notes | [C4c](./C4c-remote-files-encryption-privacy.md) (app opens every file type); library B2 (`Outline` and `NameTree` views, `LaminaId`s; [pdf-library B2](../../plans/pdf-library.md#b2-document-model)) exposed through **B6**; [C3c](./C3c-viewer-perf-harness.md) (thumbnails keep the harness's long-task budget) |
| ADRs | Follows 0004 (thumbnails rendered through `RenderBackend` at low priority), 0009, [0005](../../decisions/0005-document-model.md) §4–5 (typed views, stable IDs) |
| Branch | `feat/c6a-thumbnails-outline` |
| PR title | `feat(ui): add thumbnails and outline sidebars with page labels and go-to-page` |
| Size | L (600–1,500 lines incl. tests) |
| PR | — |

## Goal

Give readers the two classic ways to move through a long document: a virtualised **thumbnails** sidebar rendered at low priority, and an **outline** (bookmarks) sidebar with expand/collapse that tracks the current section. Page numbers everywhere show the document's **page labels** (`i, ii, 1, 2…`, ISO 32000-2 §12.4.2) and the page indicator accepts either a label or a physical number for go-to-page.

## Context

- [UI/UX plan §5.1](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) — thumbnails and outline are tabs in [U1c](./U1c-inverted-l-shell-and-modes.md)'s left rail (and in U3a's bottom sheets on phones), not a sidebar of their own; this todo fills those slots.
- [04 §1 navigation](../../research/pdf/04-interactive-security.md#1-navigation) — outlines (§12.3.3), destinations (§12.3.2), page labels (§12.4.2) as a number tree in `/PageLabels`.
- [rendering notes §4](../../research/architecture/notes/rendering.md) — thumbnails at low DPI in a separate lower-priority queue.
- [web-app.md §4 functional 6](../../plans/web-app.md#functional).
- [C3a](./C3a-viewer-worker-rpc.md) priorities (`thumbnail`), [C3b](./C3b-viewer-virtualiser-and-zoom.md) virtualiser patterns and `goToPage`.
- `fixtures/corpus/expected.json` — outline facts from pypdf (ADR 0018 tier 4); B2 adds outline titles.

## Scope

**In scope**

- RPC: `outline() → OutlineItem[]` (title, resolved destination `{ page, x?, y?, zoom?, fit }`, children, open/closed state from `/Count`, colour/style flags, stable `LaminaId`), `pageLabels() → string[]` (or a compact range form), `resolveDestination(dest | name)`.
- If B2's views don't expose page labels or destination resolution, add read-only accessors in `lamina-model`/`lamina-wasm`/`@lamina/pdf` in this PR (small; coordinate with the library owner) — page-label number tree walking with `/S` styles D, R, r, A, a, `/P` prefix, `/St` start.
- `@lamina/viewer`: `ThumbnailStrip` (framework-free virtualised list using the C3b layout code at thumbnail scale, own small canvas pool, `thumbnail` priority, cancels when scrolled away, current page highlighted), destination navigation (`goToDestination(dest)` with fit modes XYZ/Fit/FitH/FitV/FitR/FitB mapped to zoom + scroll).
- `@lamina/ui`: `Sidebar` with tabs (Thumbnails, Outline), resizable and collapsible, remembered per session; `OutlineTree` (WAI-ARIA tree pattern: arrow keys, Home/End, expand/collapse, type-ahead), current-section tracking (deepest item whose destination is at or before the top of the viewport); page indicator shows labels (`iv (4 / 120)`) and go-to-page accepts labels or numbers (ambiguity: label wins, show both in a hint).
- `page=N` and `nameddest=<name>` fragment keys (C4c's fragment format, parsed by `parseFragment` and written by `buildFragment`, so they combine with an encoded `url`, e.g. `#url=<encoded>&page=5`) navigate after load; a change to only these keys navigates within the open document (C4c's `hashchange` rule).

**Out of scope** (and which todo covers it)

- Link annotations, named destinations from links, back/forward history → [C6b](./C6b-links-and-history.md) (this PR's `goToDestination` is reused).
- Editing the outline (editor E3).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Check what B2/B6 expose (`Outline`, `NameTree`, destinations, page labels); add missing read-only accessors with Rust unit tests and TS bindings (ISO 32000-2 §12.3.2, §12.3.3, §12.4.2 cited in doc comments).
- [ ] RPC methods and worker implementation; outline loaded lazily when the sidebar opens (big outlines can be thousands of items) with a child-on-demand option.
- [ ] Destination → viewport mapping with unit tests for each fit type and for rotated pages.
- [ ] `ThumbnailStrip`: ~120 px wide renders, aspect from page sizes, LRU bitmap cache with its own budget (e.g. 32 MB), click/Enter navigates, keyboard roving focus, `aria-label` with the page label.
- [ ] `OutlineTree` component (React, primitives from C4a), virtualised if > 500 items; current-section tracking subscribed to viewer state with throttling.
- [ ] Page-label support in the page indicator and in thumbnails; go-to-page parsing.
- [ ] Facts: extend `expected.json` (via the independent tool, pypdf/pdf.js, never lamina) with outline destinations (page index) and page labels for corpus files that have them; add a fixture with roman/arabic labels and a nested outline (generated with `@lamina/pdf` builder per B5 — builder already supports outlines and page labels).
- [ ] Playwright specs.

## Acceptance criteria

- [ ] **Outline targets match `expected.json`**: for every corpus file with an outline, each item's resolved page index equals the facts; titles match.
- [ ] Page labels match facts on labelled fixtures; go-to-page by label and by number both work; an invalid entry is announced and does not navigate.
- [ ] Thumbnails sidebar on the 1,000-page fixture never mounts more than visible ± margin thumbnails, renders at `thumbnail` priority (main view renders preempt it, asserted via scheduler events), and keeps the C3c long-task budget.
- [ ] Outline tree passes the WAI-ARIA tree keyboard interactions in Playwright and has zero serious axe violations.
- [ ] Current-section tracking highlights the correct outline item while scrolling a nested-outline fixture.
- [ ] `#page=5`, `#nameddest=<name>` and `#url=<encoded>&page=5` deep links land on the right page.

## Post-merge checks

None.

## Tests

Rust unit tests for any new model accessors (page-label styles, destination resolution); Vitest unit tests for destination mapping and label parsing; Vitest browser tests for `ThumbnailStrip`; Playwright `apps/web/e2e/navigation-sidebar.spec.ts`; facts from independent tools.

## Docs to update

`packages/viewer/README.md`, `packages/ui/README.md`, `fixtures/README.md` (new fixture provenance, new facts); tick "Thumbnails sidebar…", "Outline sidebar…" and "Page labels in the page indicator and go-to-page" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Library accessors**: page labels and destination resolution may not exist after B2; this PR adds them minimally — agree with the library owner to avoid duplicate work in E3.
- **Broken outlines** (cycles, invalid `/Count`, dangling destinations) are common; resolution must be tolerant (ADR 0003) and bounded (cycle detection, depth limit).
- **Thumbnail cost on huge scans**: render at very low DPI and cap concurrency; consider using embedded `/Thumb` images when present (later).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
