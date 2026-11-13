# E4i — The viewer's annotation layer

| Field | Value |
| --- | --- |
| ID | `E4i` |
| Status | `ready` |
| Plan | [E4 plan §5](../../plans/editor/e4-markup-and-drawing-annotations.md#5-the-viewer-while-drawing) ([full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations)) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E1g](../done/E1g-redraw-after-edit.md), [E4a](../done/E4a-markup-and-drawing-annotations-feature-plan.md), [E4b](../done/E4b-annotation-model-and-overlay.md) |
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
- A gesture API the tools of [E4f](E4f-annotation-tools-ui.md) use: start, update, commit, cancel; commit applies the op and swaps the preview for the new bitmap in one frame
- The cost of the extra render measured against E1's 150 ms budget in the viewer's performance harness

**Out of scope** (and which todo covers it)

- The tools and the inspector → [E4f](E4f-annotation-tools-ui.md)

**Editor facts**

- **Crates and packages:** `@laminapdf/viewer`, `laminapdf-render`, `@laminapdf/render`, `@laminapdf/render-pdfium`, `@laminapdf/render-hayro`
- **`SaveConstraint`:** none (no op of its own)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] `pageAnnotations` in the worker
- [ ] The layer, hit-testing, handles and previews
- [ ] `hidden_annotations` in the adapters
- [ ] The gesture API
- [ ] The performance case

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

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
