# E4i — The viewer's annotation layer

| Field | Value |
| --- | --- |
| ID | `E4i` |
| Status | `done` |
| Plan | [E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1g](./E1g-redraw-after-edit.md), [E4a](./E4a-markup-and-drawing-annotations-feature-plan.md), [E4b](./E4b-annotation-model-and-overlay.md) |
| ADRs | Follows [0004](../../decisions/0004-render-backend.md), [0009](../../decisions/0009-web-client-architecture.md) |
| Branch | `feat/e4i-viewer-annotation-layer` |
| PR title | `feat(viewer): the annotation layer, with previews and one-annotation hiding` |
| Size | L (400–800): the layer, hit-testing and handles, and one new render request field in three adapters |
| PR | — |

## Goal

An annotation layer in the viewer's `annot` slot that reads each page's annotation records, hit-tests them, draws selection handles and gesture previews, and asks the renderer to leave out the annotation being moved or edited, so the page never shows it twice ([E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing)).

## Context

- [E4 plan §2.3](../../plans/editor/e4-markup-and-drawing-annotations.md#23-reading-annotations) (the records) and [§5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing)
- [E1g](../done/E1g-redraw-after-edit.md): the redraw after an edit
- **Design (Paper):** [C02 Ink and shapes](../../design/paper/comment-review.md#c02-ink-and-shapes) (handles, keyboard moves) and design language §7.32

## Scope

**In scope**

- The worker's `pageAnnotations(page)` method over the façade's records, cached per page revision
- `AnnotationLayer` in the `annot` slot: hit-testing, selection and multiple selection, the handles of §7.32, SVG previews while drawing, moving and resizing, each annotation a button in reading order with the keyboard moves of C02
- `RenderRequest::hidden_annotations` in `laminapdf-render` and `@laminapdf/render`; PDFium finds the annotation by its index in the page's `/Annots` (carried in the record) and sets the Hidden flag on the loaded page for that render only; a render with hidden annotations has its own view key and never enters the bitmap cache; hayro honours it if its renderer can, and the adapter's docs say whether it does
- A gesture API the tools of [E4f](../ready/E4f-annotation-tools-ui.md) use: start, update, commit, cancel; commit applies the op and swaps the preview for the new bitmap in one frame
- The cost of the extra render measured against E1's 150 ms budget in the viewer's performance harness

**Out of scope** (and which todo covers it)

- The tools and the inspector → [E4f](../ready/E4f-annotation-tools-ui.md)

**Editor facts**

- **Crates and packages:** `@laminapdf/viewer`, `laminapdf-render`, `@laminapdf/render`, `@laminapdf/render-pdfium`, `@laminapdf/render-hayro`
- **`SaveConstraint`:** none (no op of its own)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [x] `pageAnnotations` in the worker
- [x] The layer, hit-testing, handles and previews
- [x] `hidden_annotations` in the adapters
- [x] The gesture API
- [x] The performance case

## Acceptance criteria

- [ ] Playwright in Chromium, Firefox and WebKit: dragging an annotation on `fixtures/annotations/every-subtype.pdf` shows it once during the drag and once after release, and the keyboard moves it 1 pt and 10 pt
- [ ] The render contract suite covers `hidden_annotations` for both adapters, including two annotations with the same `/NM`
- [ ] A cancelled drag shows the annotation again at once, and the thumbnail strip never shows the page without it
- [ ] The extra render during a drag stays within E1's budget in the performance harness, or the PR records the measurement and a follow-up

## Post-merge checks

None.

## Tests

- Vitest for hit-testing and the gesture state machine
- The render contract suite in Node and browsers
- Playwright specs in `packages/viewer/test/browser/`

## Docs to update

- `@laminapdf/viewer` API docs: the annotation layer
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and the [E4 plan §8](../../plans/editor/e4-markup-and-drawing-annotations.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- Setting a flag on PDFium's loaded page may need the page reloaded after the render; measure it

## As built

- **The render request.** `RenderRequest::hidden_annotations` (`hiddenAnnotations` in TypeScript) lists places in the page's `/Annots`. In Rust the request is no longer `Copy`, since the list is a `Vec`: callers that reused a request clone it, and the changeset says so. Both PDFium adapters, native and WebAssembly, set the Hidden flag on the loaded page's annotation and clear it after the render, since the flag lands on the document's own objects and PDFium reads it afresh at every render; no page reload was needed. The recolouring decision (which annotation rectangles keep their colours) is made before anything is hidden, so its cache stays right.
- **A flag that won't clear fails the render.** Both PDFium adapters clear the Hidden flags after the render and fail it with `render-failed` (stage "show annotation") when one won't clear, so an annotation never stays hidden in later renders unsaid; the render's own error, if it has one, wins. No test covers that failure: PDFium clears the flag of any annotation it could hide, and the adapters offer no way to make it refuse.
- **hayro honours it, approximately.** hayro draws every annotation or none and exposes no per-annotation filter, so its adapter draws the tile again without annotations and copies that drawing over each hidden annotation's `/Rect`. An annotation that overlaps a hidden one is left out inside that rectangle too; the adapter's docs say so. The contract suite's fixture, `fixtures/annotations/same-name.pdf`, has two squares with the same `/NM` that don't overlap, and a third already Hidden.
- **The layer is opt-in.** `AnnotationLayer` is a class the app adds with `viewer.addLayer` while an editing mode is on, not a `createViewer` option. Its buttons take the pointer from the text under them, and the reader's startup script never loads it. It shares the `annot` slot with the link layer.
- **The viewer's part.** `viewer.hideAnnotations(page, places)` and `viewer.whenDrawn(page)` are public. A page with hidden annotations renders under its own view key, never enters the bitmap cache, and keeps its backdrop as it was (only the part in view is drawn without the annotation, so a drag never costs a whole-page render). An edit to the page clears its hidden list. While someone waits on `whenDrawn`, the page's new tiles are fetched first and drawn in one task, so the preview disappears in the same frame that the page's new pixels appear.
- **Resizing comes with the tools.** The handles of §7.32 are drawn round the selection, and the gesture API previews any change, but dragging a handle doesn't resize yet, and Alt+arrows, C02's resize from the bottom-right handle, does nothing: a resize is a spec patch per subtype. [E4f](../ready/E4f-annotation-tools-ui.md)'s scope now holds resizing through the handles and `startGesture`, Shift keeping proportions, Alt+arrows and the 24 px hit areas. Moves, by drag and by the arrow keys, are built in: one edit and one undo step, with an `updateAnnot` and its `translate` for each annotation moved.
- **Places can go stale.** A gesture hides annotations by their places in `/Annots`. Before a drag's move is applied, the layer reads the page's records again and cancels the drag if any moved annotation is no longer at its place, as after an undo or another tool's edit during the drag.
- **The desktop engine.** The desktop client's `pageAnnotations` answers an empty list, as its `pageLinks` does, and its tile protocol carries no hidden list yet: the desktop part is E4h's.
- **The reader's startup budget.** The viewer's hiding and `whenDrawn` ship in the reader's startup script though the reader adds no annotation layer: a fresh `cargo xtask web build` measured 285,549 bytes gzipped, 49 over the budget, which rose to 286,050. A hook that loads the hiding with the layer's module would still need its calls in the paint loop (the drawing of tiles together, the settle points, the cache bypass), so it would save little of the code and split the paint loop in two; the code stays in the viewer.
- **The measurement.** The perf harness's new case hides the annotation of the page in view on a 100-page file with one on every page (`marked`), nine times. In Chromium on the owner's Mac (`cargo xtask viewer-perf chromium --quick`, 2026-11-16), the page in view was drawn without it in a median 25 ms and with it again in 5 ms, against the edit's 150 ms budget.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
