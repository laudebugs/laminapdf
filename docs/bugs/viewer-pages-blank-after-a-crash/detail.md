# Pages and thumbnails stay blank after the document worker crashes

[Resolution](resolution.md).

## What we saw

On laudebugs/lamina#445's CI (2026-11-12), the job "@laminapdf/sdk in Chromium and Firefox" failed in Firefox although all 127 of `@laminapdf/viewer`'s browser tests passed. Vitest caught one unhandled error, `LaminaPdfError: the worker stopped`, during `edit.browser.test.ts`'s "reopens the saved file after a restart, and takes back only the edits since the save". That test came in with laudebugs/lamina#444, and #445 changed no viewer code. On a rerun the job passed.

The test opens a viewer and a thumbnail strip, then crashes the document worker on purpose and checks that the client restarts it.

## How to reproduce

It needs a render to be waiting when the worker crashes, so the test above fails only now and then. Holding the worker makes it certain:

1. Open a long document in a viewer and a thumbnail strip on the test worker (`testViewer()`).
2. Hold the worker (`__busy`), then scroll the strip and the main view, so new pages queue their renders behind the hold.
3. Crash the worker (`__crash`). It runs at document priority, so it goes before the queued renders, and the crash rejects them.

After the client restarts the worker, the thumbnails and pages that were waiting stay blank, and the strip reports one error per thumbnail to the window.

## Cause

When the worker crashes, the SDK's worker client rejects every call still waiting on it with `worker-crashed` ("the worker stopped"). `ViewerClient` then restarts the worker and reopens the document. Neither view knew that a crash isn't a failed render.

- **The thumbnail strip** (`thumbnails/strip.ts`) treated the rejection like any failed render: it marked the thumbnail done, so it is never asked for again while mounted, and passed the error to `reportError`. That browser function reports the error to the window as unhandled, which is what Vitest counted.
- **The main view** (`view/viewer.ts`) set the page to `error`, and `#scheduleView` skips a page in `error`. After the restart, `#reopened` redraws only the edited pages, so a page interrupted by the crash stayed blank until the zoom changed.

So a reader whose worker crashes sees the document reopen with some pages and thumbnails missing. The flaky test showed the bug only because it crashes the worker while the strip is drawing.

Ending the worker on purpose is different. `ViewerClient.stop()` (the reader's Stop, or a render past the hard cap) ends it through the SDK's `terminate()`, which rejects the waiting calls with the reason `terminated`. Drawing those renders again would run the slow render that was stopped, so they stay errors.
