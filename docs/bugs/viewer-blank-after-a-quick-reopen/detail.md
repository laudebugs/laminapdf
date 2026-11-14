# The view stays blank after a quick reopen

## What happened

After a document was opened with another one already open, the reader sometimes showed an empty view: the title and the announcement named the new file, and no page was ever drawn. The reader's close tests (`apps/web/e2e/close.e2e.test.ts`) caught it in Chromium on CI, where the step "More › Open another file…" timed out waiting for a page canvas on each of the last three runs on `main`, its retry included ([#578](https://github.com/laudebugs/laminapdf/issues/578)).

## How to reproduce it

On a fast enough open:

1. Open a document in the reader.
2. Open a document again (More › Open another file…, or any open) so that it opens before the browser draws its next frame.
3. The view is empty, and stays empty until the window is resized.

It never happened on a development machine: nine runs of the close tests alone, three of them on one CPU core, and the whole Chromium e2e suite all passed. The viewer's new browser test reproduces it every time by hiding and showing the view in one task.

## Slow or stuck

Stuck. CI's failure, with the e2e helpers reporting what the page showed at the timeout, after 30 seconds:

| What | Value |
| --- | --- |
| Window title | `Outline fixture · Lamina PDF` |
| Announced | "Opened outline.pdf, 3 pages" |
| The view's box | shown (`hidden` false), no canvas in it |
| Console | no error |

So the reader had reached its open state and shown the view, and the viewer had drawn nothing into it.

## Cause

The reader hides the view while a document opens (`phase` `opening`) and shows it once the open is done. A document opened into a hidden view can't be placed, so the viewer marks it pending (`#pending`) and waits for its `ResizeObserver` to report a size, then lays it out.

A `ResizeObserver` reports a size only when it differs from the one it last observed at a rendering step. When the open finished before the browser's next frame, the view went from shown to hidden and back between two frames: at the next frame it had the size it had before, the observer reported nothing, and the pending view was never placed. The frame the viewer had scheduled on open skipped its update while the view was pending, so nothing else placed it either.

The open's speed decides it: a reopened local file is in the worker's reach at once, and CI's headless browser draws frames less often than a development machine.

## Not yet explained

The close tests' other failure on `main`, "asks before the address drops `#url=`" (the unsaved-changes dialog never appeared after the address lost its link), has not failed since the e2e helpers began reporting the page's state. It may have the same cause or another; the next failure will say what the reader showed.
