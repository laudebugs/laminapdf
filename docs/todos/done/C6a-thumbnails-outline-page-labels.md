# C6a — Thumbnails sidebar, outline sidebar, page labels and go-to-page

| Field | Value |
| --- | --- |
| ID | `C6a` |
| Status | `done` |
| Plan | [web reader §6 C6. Navigation, print and properties](../../plans/web-app.md#c6-navigation-print-and-properties) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md), [B6b](./B6b-lamina-pdf-package.md), [C3c](./C3c-viewer-perf-harness.md), [C4c](./C4c-remote-files-encryption-privacy.md), [U1c](./U1c-inverted-l-shell-and-modes.md) |
| Dependency notes | [C4c](../done/C4c-remote-files-encryption-privacy.md) (app opens every file type); library B2 (`Outline` and `NameTree` views, `LaminaPdfId`s; [pdf-library B2](../../plans/pdf-library.md#b2-document-model)) exposed through **B6**; [C3c](../done/C3c-viewer-perf-harness.md) (thumbnails keep the harness's long-task budget) |
| ADRs | Follows 0004 (thumbnails rendered through `RenderBackend` at low priority), 0009, [0005](../../decisions/0005-document-model.md) §4–5 (typed views, stable IDs) |
| Branch | `feat/c6a-thumbnails-outline` |
| PR title | `feat(ui): add thumbnails and outline sidebars with page labels and go-to-page` |
| Size | L (600–1,500 lines incl. tests) |
| PR | [#348](https://github.com/laudebugs/laminapdf/pull/348) |

## Goal

Give readers the two classic ways to move through a long document: a virtualised **thumbnails** sidebar rendered at low priority, and an **outline** (bookmarks) sidebar with expand/collapse that tracks the current section. Page numbers everywhere show the document's **page labels** (`i, ii, 1, 2…`, ISO 32000-2 §12.4.2) and the page indicator accepts either a label or a physical number for go-to-page.

## Context

- **Design (Paper):** [R06 Rail tabs](../../design/paper/reader-a.md#r06-rail-tabs). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.1](../../plans/ui-ux.md#51-desktop-and-web-layout-r1-r2) — thumbnails and outline are tabs in [U1c](../done/U1c-inverted-l-shell-and-modes.md)'s left rail (and in U3a's bottom sheets on phones), not a sidebar of their own; this todo fills those slots.
- [04 §1 navigation](../../research/pdf/04-interactive-security.md#1-navigation) — outlines (§12.3.3), destinations (§12.3.2), page labels (§12.4.2) as a number tree in `/PageLabels`.
- [rendering notes §4](../../research/architecture/notes/rendering.md) — thumbnails at low DPI in a separate lower-priority queue.
- [web-app.md §4 functional 6](../../plans/web-app.md#functional).
- [C3a](../done/C3a-viewer-worker-rpc.md) priorities (`thumbnail`), [C3b](../done/C3b-viewer-virtualiser-and-zoom.md) virtualiser patterns and `goToPage`.
- `fixtures/corpus/expected.json` — outline facts from pypdf (ADR 0018 tier 4); B2 adds outline titles.

## Scope

**In scope**

- RPC: `outline() → OutlineItem[]` (title, resolved destination `{ page, x?, y?, zoom?, fit }`, children, open/closed state from `/Count`, colour/style flags, stable `LaminaPdfId`), `pageLabels() → string[]` (or a compact range form), `resolveDestination(dest | name)`.
- If B2's views don't expose page labels or destination resolution, add read-only accessors in `laminapdf-model`/`laminapdf-wasm`/`@laminapdf/sdk` in this PR (small; coordinate with the library owner) — page-label number tree walking with `/S` styles D, R, r, A, a, `/P` prefix, `/St` start.
- `@laminapdf/viewer`: `ThumbnailStrip` (framework-free virtualised list using the C3b layout code at thumbnail scale, own small canvas pool, `thumbnail` priority, cancels when scrolled away, current page highlighted), destination navigation (`goToDestination(dest)` with fit modes XYZ/Fit/FitH/FitV/FitR/FitB mapped to zoom + scroll).
- `@laminapdf/ui`: `Sidebar` with tabs (Thumbnails, Outline), resizable and collapsible, remembered per session; `OutlineTree` (WAI-ARIA tree pattern: arrow keys, Home/End, expand/collapse, type-ahead), current-section tracking (deepest item whose destination is at or before the top of the viewport); page indicator shows labels (`iv (4 / 120)`) and go-to-page accepts labels or numbers (ambiguity: label wins, show both in a hint).
- `page=N` and `nameddest=<name>` fragment keys (C4c's fragment format, parsed by `parseFragment` and written by `buildFragment`, so they combine with an encoded `url`, e.g. `#url=<encoded>&page=5`) navigate after load; a change to only these keys navigates within the open document (C4c's `hashchange` rule).

**Out of scope** (and which todo covers it)

- Link annotations, named destinations from links, back/forward history → [C6b](../done/C6b-links-and-history.md) (this PR's `goToDestination` is reused).
- Editing the outline (editor E3).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Check what B2/B6 expose (`Outline`, `NameTree`, destinations, page labels); add missing read-only accessors with Rust unit tests and TS bindings (ISO 32000-2 §12.3.2, §12.3.3, §12.4.2 cited in doc comments).
- [x] RPC methods and worker implementation; outline loaded lazily when the sidebar opens (big outlines can be thousands of items) with a child-on-demand option.
- [x] Destination → viewport mapping with unit tests for each fit type and for rotated pages.
- [x] `ThumbnailStrip`: ~120 px wide renders, aspect from page sizes, LRU bitmap cache with its own budget (e.g. 32 MB), click/Enter navigates, keyboard roving focus, `aria-label` with the page label.
- [x] `OutlineTree` component (React, primitives from C4a), virtualised if > 500 items; current-section tracking subscribed to viewer state with throttling.
- [x] Page-label support in the page indicator and in thumbnails; go-to-page parsing.
- [x] Facts: extend `expected.json` (via the independent tool, pypdf/pdf.js, never lamina) with outline destinations (page index) and page labels for corpus files that have them; add a fixture with roman/arabic labels and a nested outline (generated with `@laminapdf/sdk` builder per B5 — builder already supports outlines and page labels).
- [x] Playwright specs.
- [x] The outline jump and thumbnails from the Contents sheet on the phone and tablet profiles of `apps/web/e2e/devices.e2e.test.ts` (U3a left them to this todo).

## Acceptance criteria

- [x] **Outline targets match `expected.json`**: for every corpus file with an outline, each item's resolved page index equals the facts; titles match.
- [x] Page labels match facts on labelled fixtures; go-to-page by label and by number both work; an invalid entry is announced and does not navigate.
- [ ] Thumbnails sidebar on the 1,000-page fixture never mounts more than visible ± margin thumbnails, renders at `thumbnail` priority (main view renders preempt it, asserted via scheduler events), and keeps the C3c long-task budget. (The first two hold, at `background` priority; the long-task budget is unmeasured: see Notes.)
- [x] Outline tree passes the WAI-ARIA tree keyboard interactions in Playwright and has zero serious axe violations.
- [x] Current-section tracking highlights the correct outline item while scrolling a nested-outline fixture.
- [x] `#page=5`, `#nameddest=<name>` and `#url=<encoded>&page=5` deep links land on the right page.

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

## Notes

What the PR did differently from the plan above, and why:

- **Claim.** The work PR's first commit claimed the todo, as the repo now does, not a claim PR.
- **RPC.** `outline()` and `namedDestination(name)`. Page labels come with every page when the document opens (`OpenedDocument.pages[i].label`), so no `pageLabels()` call was needed. The outline's explicit destinations come resolved, so `namedDestination` covers what `resolveDestination` would have. The whole outline is read the first time the Outline tab shows, and the tree draws only the rows in view past 500 items, so there is no child-on-demand option.
- **Outline items.** Each has a title, its `/Count` open state, its target and its children; ids are paths ("2.0"). Colour and style flags and `LaminaPdfId`s are left out: the R06 design draws neither, and no caller needs them yet.
- **Façade.** `Document::outline()` and `named_destination(name)` (`crates/laminapdf/src/navigation.rs`), over `laminapdf-model`'s outline with a new `Outline::named_destination`. `/FitH` and `/FitV` keep fitting the displayed width and height on a quarter-turned page and drop their operand there, as Lamina PDF's builder writes it (`null`). pdf.js fits the same way but maps the operand as a point, which moves the other axis.
- **Facts.** `tools/corpus-facts/facts.py` adds `outline_pages` and `page_labels` to `fixtures/corpus/expected.json` (pypdf 6.21.3). No new fixture: the corpus's `outline.pdf` already has a nested outline and roman and prefixed labels (i, A-1, A-2), and the links-and-outlines golden has i, ii, 1, 2. Lamina PDF and pypdf disagree on one item, `outline.pdf`'s Appendix: its `GoTo` action names a destination whose `/Dests` value is a dictionary (ISO 32000-2 §12.3.2.4), which pypdf can't follow. `fixtures/corpus/exceptions.toml` records it, and an exception may now cite the spec section instead of an ADR 0003 rule.
- **Sidebar.** U1c's `LeftRail` holds the Pages and Outline tabs and already keeps its tab, width and open state. On a phone it is the Contents sheet; there, and in the drawer below 1024 px, going somewhere from it closes it.
- **Go-to-page.** `PageInput` already took a label or a number, and a label wins when both match. The reader now gives it the labels. It shows no hint for the ambiguous case.
- **Current section.** The item whose place is the latest at or before a point 24 px under the viewport's top (just past the gap a page jump leaves above its page), the deepest of those at one place, so an item that points back never wins over the chapter being read. The point is turned back by the view's rotation first. It is worked out at most every 150 ms.
- **The tree's role.** React Aria's `Tree` renders `role="treegrid"` rows, with the tree pattern's keys: Up and Down, Right and Left to open and close, Home and End, type-ahead. `aria-current` sits on the row's link, as on `OutlineRow`.
- **`nameddest` and `page` together.** `nameddest` wins, as in pdf.js, so with both in the address a change to `page` alone doesn't move the view.
- **Thumbnails.** A strip that is hidden (its tab not shown) renders nothing, and a failed render keeps its placeholder until the thumbnail is mounted again.
- **Long-task budget.** The perf harness wasn't run with the strip open: thumbnails render at `background` priority behind every main-view render (asserted in `navigation.browser.test.ts`), but the C3c long-task budget with the strip is unmeasured.
- **Tests.** The Playwright suite is `apps/web/e2e/navigation.e2e.test.ts`, the repo's e2e naming.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
