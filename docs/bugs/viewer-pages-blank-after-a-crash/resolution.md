# Resolution: pages and thumbnails stay blank after the document worker crashes

[Diagnosis](detail.md).

## The fix

`ViewerClient`'s module gains `interruptedByCrash(error)`: true for a `worker-crashed` rejection whose reason isn't `terminated`, that is, a worker that crashed rather than one the client ended. Both views use it, so the rule has one home.

- **The thumbnail strip** leaves an interrupted thumbnail drawable and schedules a frame, instead of marking it done and reporting the error.
- **The main view** leaves an interrupted page as it was instead of setting `error`. Its pass's `finally` schedules the next frame, which draws the page again.

A render asked for while the worker restarts waits for the restart (`#recovering`) before it reads the document's ID, so the retry runs once, in the new worker. The client rejects the waiting calls and starts the restart in the same step, so the restart is already under way when the views retry.

A render stopped on purpose (`stop()`: the reader's Stop, or the hard cap) still ends in `error`, so a page that hangs the worker isn't drawn again in a loop.

## How it was checked

- **Two regression tests:**
  - "draws the thumbnails a crash interrupted once the worker restarts, and reports no error", in `navigation.browser.test.ts`;
  - "draws the pages a crash interrupted once the worker restarts, leaving none in error", in `viewer.browser.test.ts`.

  Each holds the worker, scrolls so renders queue, crashes the worker, and waits until every mounted thumbnail or page is drawn again.
- **Against the old code, both fail.** With `main`'s strip, the crash rejected 13 thumbnail renders with "the worker stopped", and the window got `Uncaught LaminaPdfError: the worker stopped`, the error CI saw. With `main`'s page view, the pages the crash interrupted never left `error`. With the fix, both pass; the page view's log showed five renders rejected with the reason `error` and drawn again.
- **`@laminapdf/viewer`'s suites:** the browser tests in Chromium (129) and the Node tests (367) pass, as do its typecheck and Biome. CI runs the browser tests in Firefox and WebKit too.

## Left alone

- **Find** (`find/controller.ts`) also passes a failed call to `reportError`, so a search running when the worker crashes stops where it was and reports the error. Whether a search should start again after a restart is a separate change.
