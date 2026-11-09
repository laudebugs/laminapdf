# The reader's text can't be selected with the mouse

Dragging the mouse across a page's text in the reader selected nothing. The two layer slots above the text covered the whole page and took every press, so no drag reached the text's spans. The fix is in [resolution.md](resolution.md).

## What the user saw

In the web reader, dragging the mouse across a page's text selected nothing, though the viewer's text layer had shipped and its browser tests passed.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open `fixtures/golden/examples/invoice.pdf` with Choose a file.
3. Drag the mouse across a line of text, such as "Widgets, blue (W-100)".

Nothing is selected, and copying gives nothing.

## What was measured

In the built reader, in Chromium and WebKit, with Playwright's real mouse:

| | The element at the word's centre (`elementFromPoint`) | `getSelection().toString()` after the drag |
| --- | --- | --- |
| As built | `div.lamina-layer-highlight` | `""` |
| With `pointer-events: none` on the `annot` and `highlight` slots | the word's `span` | `"Widgets, blue (W-100)"` |

The text layer itself was right: one span per run, over its glyphs, with `user-select: text`. The three slots over the canvas were not:

| Slot | Order | `pointer-events` | `user-select` |
| --- | --- | --- | --- |
| `lamina-layer-text` | bottom | `auto` | `none` (inherited from the viewer's root) |
| `lamina-layer-annot` | middle | `auto` | `none` |
| `lamina-layer-highlight` | top | `auto` | `none` |

The touch gestures were ruled out: they take only fingers and pens, never the mouse (`view/touch.ts`).

## The cause

`Viewer.#slot` (`packages/viewer/src/view/viewer.ts`) gives each page three slots above its canvas, each `position: absolute; inset: 0`, so each covers the whole page. The two above the text hold nothing a reader clicks: find's highlight boxes, the only content so far, already let the pointer through. The slots themselves took every press. A drag started on the top slot, which inherits `user-select: none` from the viewer's root, so the browser began no selection.

The viewer's text-layer tests selected text through the Selection API (`selectAllChildren`, `addRange`), which never asks which element is under the pointer, so they passed while no mouse drag could reach a span.
