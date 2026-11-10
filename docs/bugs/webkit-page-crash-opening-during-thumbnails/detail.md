# WebKit's page crashes when a document opens while thumbnails are still rendering

Status: open. The cause isn't found yet; this records what is known.

## What happens

The viewer's WebKit browser tests (`navigation.browser.test.ts`, `viewer.browser.test.ts`) end with "The webkit page crashed while running tests" every time in Playwright's WebKit on Linux, and now and then on CI's macOS runners, where another file running alongside (`rpc.browser.test.ts`) is often the one reported. The machine has plenty of free memory, and WebKit leaves no crash report: the browser exits cleanly.

## Smallest reproduction

In one page, with the 1,000-page document from `navigation.browser.test.ts`:

1. Open it in a viewer with a thumbnail strip, and scroll the strip from top to bottom, two frames per 1,500 px.
2. Destroy the strip and the viewer.
3. Open any other document, in the same viewer client or in a new one.

Step 3 crashes the page. It doesn't crash when:

- the strip isn't scrolled, or only the main view is scrolled through the 1,000 pages;
- step 3 waits 3 s, or waits for one more answer from the worker first;
- step 3 is skipped.

The viewer from before main's #383 crashes the same way, and loading the text module before the scroll changes nothing.

## Where to look next

The crash needs the strip's backlog of background renders still pending when a document opens, in any worker. The renderer copies its pixels out of PDFium's memory before making a bitmap, so a detached WebAssembly buffer isn't the cause. Next: count the bitmaps alive and the renders pending at step 3, and try a WebKit build with crash logs (or Safari on a Mac).
