# A file dropped on the home screen right after it showed was ignored

## What happened

In WebKit, the reader's e2e tests that open a file by dropping or pasting it failed at random: the home screen stayed up and no document opened. Main failed one or two of them in each of 4 runs of `reader.e2e.test.ts` in Playwright's WebKit on Linux, and CI's WebKit job failed in the same way on macOS. Opening through Choose a file never failed.

## How to reproduce it

Run `LAMINA_PDF_E2E_BROWSER=webkit pnpm --filter @laminapdf/web exec vitest run -c vitest.e2e.config.ts e2e/reader.e2e.test.ts` a few times after `cargo xtask web build`. The tests dispatch the drop as soon as the "Open a PDF" heading shows.

## Cause

`OpenDropZone` (`packages/ui/src/reader/open-drop-zone.tsx`) added its window listeners for `drop` and `paste` in a `useEffect`, which React runs after the screen paints. Between the paint and the effect, the home screen already said a dropped or pasted PDF would open, but nothing listened. On a slow start that gap is long enough to drop into. A real drop in the gap isn't prevented, so the browser opens the file itself, in place of the app; a pasted file is lost.
