# Thumbnails stayed blank after the document's worker restarted

## What happened

The viewer's browser tests failed in CI's Chromium leg on PR #456, twice in a row (Full lane run 43147069509, 2026-11-12), though every test passed:

```
Vitest caught 1 unhandled error during the test run.
LaminaPdfError: the worker stopped
 ❯ #stop ../sdk/src/worker/client.ts
 ❯ #crash ../sdk/src/worker/client.ts
 ❯ worker.onerror ../sdk/src/worker/client.ts
This error originated in "test/browser/edit.browser.test.ts" test file.
The last test to run before this error was "reopens the saved file after a restart, and takes back only the edits since the save".
```

Locally the whole Chromium suite showed it about one run in six. The edit tests alone (20 of 20, in Chromium and Firefox) never showed it, and a log of the calls still pending when the worker stopped never fired there: no call of the client's or the edit session's goes unhandled. That ruled out the worker client and the edit session, and left the strip, whose renders were the only calls in flight in the full suite's runs.

A reader sees the other half of the bug: when the document's worker crashes and restarts, the thumbnails whose renders the crash cut short stay blank placeholders until they scroll out of the strip and back.

## Reproduce

Open a long document with the thumbnail strip showing, and crash the worker while the strip's background renders are still queued (the browser tests' `__crash` hook). Every thumbnail render in flight rejects with `worker-crashed`; the strip reports each one and never draws those pages again. `navigation.browser.test.ts`'s restart test does this and failed before the fix.

## Cause

`ThumbnailStrip`'s `#draw` treated every failed render other than an abort as the page's own failure: it marked the thumbnail done, so it isn't asked for again while mounted, and passed the error to the browser's `reportError`. A crashed worker isn't a failure of the page, and the viewer restarts the worker and reopens the document at once. The strip also didn't listen for the restart, so nothing drew the thumbnails again under the reopened document's new id.

The edit test only triggered it: its crash lands while the strip from `showing()` is still rendering thumbnails after an edit, and the reported error is what Vitest counted.
