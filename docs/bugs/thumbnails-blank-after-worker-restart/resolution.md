# Resolution: the strip waits out a restart and draws again

## Fix

In `packages/viewer/src/thumbnails/strip.ts`:

- A render that fails with `worker-crashed` is left as it is, like an aborted one: not marked done and not reported.
- On the client's `restarted` event with `reopened`, the strip unmounts its thumbnails, clears its bitmap cache (its keys hold the old document id) and schedules a redraw, as the main view redraws after `#reopened`.

## Proof

- `navigation.browser.test.ts` gains "draws its thumbnails again after the worker restarts, and reports no error for the renders the crash cut short". It crashes a strip's own client while 1,000 pages' thumbnails queue, then expects every mounted thumbnail drawn after the restart and `reportError` never called. Before the fix it failed (thumbnails never drawn); after it, it passes in Chromium, Firefox and WebKit.
- The navigation and edit tests pass in all three browsers (33 of 33 each), and the whole Chromium suite passed twice with no unhandled error (130 of 130).

## Left for later

Nothing.
