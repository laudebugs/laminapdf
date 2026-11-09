# Selecting text in the reader hides it

Selecting text in the reader painted the selection colour over the words, so the selected text disappeared behind solid blocks. The selection's opaque tint was drawn on top of the page's drawing instead of under its ink. The fix is in [resolution.md](resolution.md).

## What the user saw

Once a mouse drag could select text ([text-not-selectable-in-reader](../text-not-selectable-in-reader/detail.md)), the selected lines turned into solid grey-blue blocks with no text in them. Only the unselected part of each line stayed readable.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open a page of paragraphs, such as `fixtures/reading-order/header-footer-ordered.pdf`.
3. Drag the mouse across several lines.

The selected lines show the tint and no text.

## What was measured

The browser test "leaves a selected word's ink as dark as drawn under an opaque tint" screenshots one word, counts its ink pixels (luma below 100), selects the word and counts again, with the app's tint (`#cedbdd`) as the selection colour:

| | Ink pixels before selecting | After selecting |
| --- | --- | --- |
| As built | 576 | 0 (Chromium, Firefox and WebKit) |
| With the text layer multiplied over the page | 576 | 576 or more in the run recorded here (the test asserts at least 80 % kept) |

## The cause

The design language defines the selection tint as a fill under the ink: `selection-tint` is the accent at 22 % over the paper, with page ink on it at 10.91:1 (design language §2). The reader sets it as the spans' `::selection` background (`apps/web/src/app.css`). But the spans lie in the text layer, above the canvas that draws the page, and the tint is opaque, so it was painted over the glyphs it selected.

The find highlights, which lie above the canvas too, already avoided this with `mix-blend-mode: multiply` (`layers/highlight-layer.ts`). The text layer had no such rule.

No test looked at what a selection shows. The text layer's tests checked the selected text, and the mouse-drag tests checked that a drag selects; nothing checked that the user can still read what they selected.
