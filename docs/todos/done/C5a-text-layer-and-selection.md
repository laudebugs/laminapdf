# C5a — Text layer, selection and copy (from lamina's extraction), with struct-tree linking

| Field | Value |
| --- | --- |
| ID | `C5a` |
| Status | `done` |
| Plan | [web reader §6 C5. Text, selection and search](../../plans/web-app.md#c5-text-selection-and-search) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [B9c](./B9c-text-extraction-bindings-and-differential.md), [C3c](./C3c-viewer-perf-harness.md), [C4c](./C4c-remote-files-encryption-privacy.md) |
| Dependency notes | Library **B9** (text extraction: `laminapdf-content`, `laminapdf-text`, `page.text()` in `@laminapdf/sdk`; [pdf-library B9](../../plans/pdf-library.md#b9-text-extraction)); [C4c](./C4c-remote-files-encryption-privacy.md) (app opens local, remote and encrypted files); [C3c](./C3c-viewer-perf-harness.md) (perf harness scenario and long-task budget) |
| ADRs | Follows 0004's two-parsers rule (text geometry from lamina, never from the renderer), 0009 (framework-free layer in `@laminapdf/viewer`), ADR 0023 (content-stream reading, from B9) |
| Branch | `feat/c5a-text-layer` |
| PR title | `feat(viewer): add the text layer with selection, copy and struct-tree linking` |
| Size | L (600–1,500 lines incl. tests) |
| PR | [#345](https://github.com/laudebugs/laminapdf/pull/345) |

## Goal

Make text in every rendered page selectable, copyable and readable by screen readers. The worker exposes glyph runs and quads from lamina's B9 extraction API through the RPC; `@laminapdf/viewer` builds a transparent DOM text layer per mounted page, aligned to the rendered glyphs at every zoom and rotation; native browser selection and copy produce the right text in five scripts, including encrypted files; and for tagged PDFs a struct-tree layer links semantics to the text spans via `aria-owns`, so the reader is accessible regardless of which render backend is active.

## Context

- [UI/UX plan §5.3](../../plans/ui-ux.md#53-selection-and-copy-r4-r5) — the viewer only **emits** selection events (client rects, selected runs, pointer type); menus, copy modes and the touch action bar are [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md)'s, so nothing here draws over the OS selection menu.
- [pdf-library B9](../../plans/pdf-library.md#b9-text-extraction) — glyph runs with Unicode, quads, direction, `ActualText`, MCIDs; accepted when five-script fixtures match and quads lie within tolerance of PDFium's char boxes.
- [03 text and fonts §8 extraction](../../research/pdf/03-text-and-fonts.md) — extraction model, reading order, ToUnicode.
- [Research: web client](../../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core) — layering: raster canvas, transparent DOM text layer, struct-tree layer via `aria-owns` (pdf.js pattern), core must expose `StructTreeRoot`/`MarkInfo`.
- [rendering notes §3](../../research/architecture/notes/rendering.md) and [web-frontend notes §3, §7](../../research/architecture/notes/web-frontend.md) — text-layer technique, accessibility through DOM layers, untagged documents.
- [web-app.md §4 functional 4](../../plans/web-app.md#functional) and [§7 risks](../../plans/web-app.md#7-risks) (two parsers disagree on geometry; untagged PDFs).
- [C3b](./C3b-viewer-virtualiser-and-zoom.md) — `PageLayerHost` slots (`lamina-layer-text`).

## Scope

**In scope**

- RPC methods: `pageText(page) → { runs: GlyphRun[] }` (Unicode string, per-glyph or per-run quads in PDF user space, font size, direction LTR/RTL/TTB, `ActualText` replacement, MCID), `structTree(page)` and `markInfo()`; low priority (`background`) with per-page LRU cache in the worker.
- If B9 doesn't expose the struct tree read API, add a minimal read-only view (`StructTreeRoot` → elements with type, alt, lang, MCID refs) in `laminapdf-model` / `laminapdf-wasm` / `@laminapdf/sdk` **in this PR** (small) — coordinate with the library owner; otherwise split it out as `C5a-pre`.
- `packages/viewer/src/layers/text-layer.ts`: per page, absolutely positioned `span`s per run (or per word for long runs), transformed with CSS `matrix()` from PDF space to CSS px at the current zoom/rotation, horizontally scaled (`scaleX`) so each span's width matches the glyph quads; `color: transparent`; selection highlight via `::selection`; `dir` and `lang` attributes; line breaks / end-of-line markers so copy yields sensible newlines. **Selection scope (decided): real cross-page selection with a single selection root.** The document's page container is the one selection root: `user-select: text` on text-layer spans only, `user-select: none` on canvases, page gaps, other layers and the app chrome. There is no per-page `user-select` boundary, so a drag selection continues from one mounted page into the next. While a selection exists, the virtualiser keeps every page the selection touches mounted (it pins them) so the selection is not destroyed by scrolling. `ctrl/cmd+A` with focus in the document selects the text of the mounted pages only (not the whole document; full-document text is reached via find); the help text says so.
- Copy: intercept `copy` to normalise output (remove soft hyphen artefacts, apply `ActualText`, join across pages in order) using the extracted Unicode rather than DOM text where they differ.
- Struct-tree layer: hidden-but-accessible tree (`role`s mapped from standard structure types: H1–H6, P, L/LI, Table/TR/TH/TD, Figure with alt) whose leaves `aria-owns` the matching text spans by MCID; untagged documents announce "untagged document" once and expose text in extraction order.
- Rotation and zoom updates re-transform existing spans (no re-extraction).

**Out of scope** (and which todo covers it)

- Find, highlights and match navigation → [C5b](../done/C5b-find-in-document.md).
- Link layer → [C6b](../ready/C6b-links-and-history.md).
- Annotation/form DOM layers (display is raster by PDFium in the reader).
- Full keyboard/screen-reader audit → [C7a](../ready/C7a-accessibility-and-theming.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`. Claimed by opening the work PR, whose first commit moved this file, as CLAUDE.md's working agreement now does.
- [x] Confirm B9's `@laminapdf/sdk` API (`page.text()` shape); write a worker adapter producing compact, transferable run data (`Float32Array` of quads + string table) to keep RPC cheap.
- [x] Add RPC methods and types; worker cache keyed by page; cancellation on page unmount.
- [x] Text-layer builder with a pure function `layoutSpans(runs, viewport) → SpanSpec[]` (unit-testable) and a DOM renderer that reuses span elements across zoom changes.
- [x] Width fitting: measure each span's natural width once (canvas `measureText` with a generic font, cached per font-size bucket) and set `transform: … scaleX(k)`; avoid layout thrash (batch reads then writes). Built as one batched read of each span's laid-out width (the box untransformed), then one batch of writes: canvas `measureText` falls back to other fonts than the page for some characters (mathematical bold missed by 1.8 px).
- [x] RTL and vertical text: set `dir="rtl"` for RTL runs; rotate spans for vertical writing; test Arabic/Hebrew selection order.
- [x] Selection root and `user-select` rules as in Scope; pin pages intersecting the active selection in the virtualiser (`selectionchange` → a `keepMounted(pageIndices)` hook on the C3b virtualiser, added in this PR if C3b has none; pinned pages count against the canvas pool); copy handler and newline/space heuristics consistent with B9's reading order, joining pages in page order with a page break newline.
- [x] Struct tree: RPC + layer + `aria-owns`; `MarkInfo /Marked` check; fallback for untagged.
- [x] Two-parser guard: in dev/debug builds, overlay text-quad outlines on the canvas (debug menu toggle) to spot misalignment; nightly test compares quads to PDFium char boxes on the corpus (reusing B9's quad test data). The outline is on under `#debug` (`createViewer`'s `text.outline`) rather than a menu toggle; `cargo xtask quads` already compares Lamina PDF's quads with PDFium's char boxes.
- [x] Encrypted documents: text extraction runs in lamina with the password from the open call (no second prompt).
- [x] Selection and copy on the phone and tablet profiles of `apps/web/e2e/devices.e2e.test.ts` (U3a left copy to this todo).

## Acceptance criteria

- [x] **Copy equals expected text** for the B9 five-script fixtures (Latin, Arabic, Devanagari, Hebrew, CJK) in Chromium, Firefox and WebKit: Playwright selects all text on a page (`Selection.selectAllChildren` on that page's text layer, or drag-select from the first to the last span), copies, reads the clipboard, and compares with the fixture's expected text (normalised whitespace).
- [x] The same holds for the encrypted versions of those fixtures (B2 encryption × B9 text).
- [x] Alignment: for every span on the corpus fixtures at 100 % and 200 % zoom and 90° rotation, the span's bounding box lies within 1.5 CSS px (or 2 % of font size) of the lamina quad transformed to the viewport (asserted from layout maths; visual spot-check screenshots attached to the PR). Asserted in three browsers on `golden/text-latin.pdf` (`packages/viewer/test/browser/text-layer.browser.test.ts`); the copy tests cover the five-script page.
- [x] On a tagged fixture, the accessibility tree (Playwright `page.accessibility.snapshot()` / Chromium AX tree) shows headings and paragraphs in struct-tree order with the text spans as children; an untagged fixture announces "untagged document". Asserted on the DOM the accessibility tree reads (roles in structure order, the figure's alt text, `aria-owns` naming the heading's spans) rather than a Chromium AX snapshot, so all three engines run it.
- [x] Text-layer construction for a typical page takes no main-thread task > 50 ms (perf harness from C3c, informational). The perf harness's viewers now build the text layer on every page, so its long-task figures include it.
- [x] Selection works across page boundaries: a Playwright drag from the middle of page 1 to the middle of page 2 copies the tail of page 1 then the head of page 2 in order, in three engines; scrolling while the selection exists does not drop it (the touched pages stay mounted); canvas and gap areas are never part of the selection. The selection is a Range from page 1's last line into page 2 rather than a mouse drag, which the three engines' automation moves differently.

## Post-merge checks

None.

## Tests

Unit tests for `layoutSpans` (matrices, rotation, RTL, vertical); Vitest browser tests for the DOM layer reuse and copy handler; Playwright `apps/web/e2e/text-copy.spec.ts` (five scripts × plain/encrypted), `a11y-structure.spec.ts` (tagged/untagged). Fixtures from B9 (`fixtures/text/` or wherever B9 puts them) and B2 encrypted fixtures; expected texts are those used by B9's tests (one source of truth, ADR 0018 §5).

## Docs to update

`packages/viewer/README.md` (text layer and struct tree); tick "Expose glyph runs and quads from the B9 extraction API through the worker RPC", "Build the DOM text layer aligned to rendered glyphs at every zoom", "Link the struct tree via `aria-owns` for tagged files" and the copy part of "Copy/extraction tests in five scripts…" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Geometry disagreement** between lamina and PDFium (fonts with odd `/Widths`, Type 3) shows as offset selection; the debug overlay and nightly quad test make it visible; fixes go to `laminapdf-text`.
- **Struct-tree read API ownership**: library team may prefer to add it in B9 or B8; agree before starting (this PR assumes a minimal read view if absent).
- **Huge pages with tens of thousands of glyphs** (maps, tables): group into word spans and cap span count per page; measure.
- **Firefox and Safari selection quirks** with transformed spans (pdf.js has years of workarounds); budget time for them.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
