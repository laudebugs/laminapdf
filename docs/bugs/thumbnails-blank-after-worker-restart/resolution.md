# Resolution: the strip waits out a restart and draws again

## Fix

In `packages/viewer/src/thumbnails/strip.ts`:

- A render that fails with `worker-crashed` is left as it is, like an aborted one: not marked done and not reported.
- On the client's `restarted` event, the strip drops its thumbnails and its bitmap cache. A restarted worker numbers its documents afresh, so an old key could name a page of the reopened document, edited or not. When the document came back, the strip takes the reopened document (its id and, through the viewer, its sizes with the edits replayed), lays it out and draws it again, as the main view does after `#reopened`. When it didn't (an encrypted document whose password died with the worker), the strip empties until the next `open`, so no render is asked of a document that's gone.

## Proof

- `navigation.browser.test.ts` gains two tests:
  - "draws its thumbnails again after the worker restarts, and reports no error for the renders the crash cut short" checks that thumbnails are still unrendered, crashes a strip's own client while 1,000 pages' thumbnails queue, then expects every mounted thumbnail drawn after the restart and `reportError` never called.
  - "empties itself, reporting nothing, when the restarted worker can't reopen a locked document" crashes the client of an encrypted document and expects no thumbnail left and nothing reported.
- Both failed before the fix and pass after it in Chromium, Firefox and WebKit. The navigation and edit tests pass in all three browsers (34 of 34 each), and the whole Chromium suite passed with no unhandled error (131 of 131).

## Left for later

Nothing.
