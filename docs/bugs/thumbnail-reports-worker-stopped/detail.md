# Detail: a thumbnail reports "the worker stopped" when the worker crashes mid-draw

[Resolution](resolution.md).

## What happened

The nightly run of 2026-11-13 ([run 43363388690](https://github.com/laudebugs/laminapdf/actions/runs/43363388690)) failed its `pdf-runtimes` job in Chromium. All 136 of the viewer's browser tests passed, but Vitest caught one unhandled error and failed the run:

```
LaminaPdfError: the worker stopped
 ❯ crashed ../sdk/src/worker/client.ts:142:10
 ❯ #stop ../sdk/src/worker/client.ts:307:39
 ❯ #crash ../sdk/src/worker/client.ts:313:15
 ❯ worker.onerror ../sdk/src/worker/client.ts:262:18
```

The last test to run was "reopens the saved file after a restart, and takes back only the edits since the save" (`edit.browser.test.ts`). A user would see the same error in the browser console whenever the document worker crashed while a thumbnail was drawing.

## How to reproduce it

Hold the worker on a thumbnail's render (the test worker's `__holdNextRender`), then stop or crash the worker. The strip's render rejects with `worker-crashed`, and the window gets an `error` event. The race in the nightly run is the same one: the test turns page 2, the strip starts redrawing its thumbnail, and the test crashes the worker at once.

## Cause

After a crash, ADR 0009 §3.6 has the viewer send its renders again, "except the request that was running when the worker died", so a page that crashes the engine can't loop. [`viewer-pages-blank-after-a-crash`](../viewer-pages-blank-after-a-crash/detail.md) put that rule in `ViewerClient.render`. The render that was drawing still rejects with `worker-crashed`, as the rule intends.

The thumbnail strip treats any render that rejects without being aborted as a failed page: it keeps the placeholder and passes the error to `reportError`. `reportError` raises it on the window as an unhandled error. A crash isn't the page's failure, and the client already tells of it with its `crashed` event (`restarted` for `stop()`). The main view handles the same rejection without reporting it: it marks the page `error`.
