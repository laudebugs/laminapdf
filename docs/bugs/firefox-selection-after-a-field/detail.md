# In Firefox, the reader stops selecting text once a text field has had the focus

In Firefox, a mouse drag across a page's text selected nothing after the user had focused the reader's page box or any other text field. Typing a page number and pressing Enter was enough: from then on, no drag in the document selected anything until the page was reloaded. Chromium and WebKit kept selecting. The fix is in [resolution.md](resolution.md).

## What the user saw

A user typed a page number into the page box, pressed Enter and tried to select a sentence on that page. The drag left no selection. Scrolling, clicking elsewhere and pressing Tab out of the box didn't bring selection back.

## How to reproduce it

1. Build the reader (`cargo xtask web build`) and serve `apps/web/dist/`.
2. Open `fixtures/corpus/outline.pdf` in Firefox.
3. Type `A-1` in the page box and press Enter.
4. Drag the mouse across the words "Page 2".

Nothing is selected.

## What was measured

Throwaway e2e probes drove the built reader with Playwright's mouse in Firefox and Chromium. Each read `getSelection()` after a drag across one span:

| Before the drag | Firefox | Chromium |
| --- | --- | --- |
| Nothing | "Page 1" | "Page 1" |
| Page box: type `A-1`, Enter | "" | "Page 2" |
| Page box: focus, then Escape | "i" | "Page 1" |
| A plain `<input>` added by the probe: type, click away | "" | "Page 1" |
| Any of the above, then clear the selection, blur, click the span, or move the selection into the viewer | "" | "Page 1" |

The "i" is the page box's own value, the first page's label: while a field has the focus, Firefox's `getSelection()` reports the field's selected text, so the drag left the document's selection empty there too. The element under the pointer was the span each time, so nothing covered the text. The drag moved the focus to the viewer and left a collapsed selection on the span's text node.

The same steps in the viewer's own browser tests, with no app around it, failed in Firefox alone. Changing one style at a time there found the trigger:

| The viewer's `user-select` | Firefox after a field had the focus |
| --- | --- |
| `none` on the root, `text` on the spans (as built) | Nothing selected |
| `text` also on the text layer's box | Nothing selected |
| `none` on the root, `text` on every element from the root to the spans | Nothing selected |
| `auto` on the root, `none` on the page shell | Nothing selected |
| `auto` on the root and every ancestor of the spans; `none` on the canvases and the other slots | The word is selected |

## The cause

The text layer made the viewer's root `user-select: none` so that only the spans, set back to `text`, could be selected. A `none` root with `text` spans is the usual way to keep canvases and gaps out of a selection, and it worked in every engine until a text field took the focus. After that, Firefox began a mouse selection on a span but didn't extend it while any ancestor was `user-select: none`, even when every element between the root and the spans was set back to `text`. Chromium and WebKit extend it, so this looks like a Firefox bug; the table above is as far as these tests narrow its trigger.

No test selected text after a field had had the focus. The viewer's drag tests ran in a fresh page, and the reader's e2e tests never combined the page box with a drag.
