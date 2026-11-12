# Resolution: pages and thumbnails stay blank after the document worker crashes

[Diagnosis](detail.md).

## The fix

`ViewerClient.render` now follows ADR 0009 §3.6 itself: after a crash, the viewer re-requests its tiles "except the request that was running when the worker died". The rule lives in the client, so every caller of `render` follows it, and the two views are unchanged.

- **A render that was waiting** when the worker was lost is sent again, once, in the restarted worker. Its caller gets the bitmap and never sees the crash. `#render` marks it interrupted when it rejects with `worker-crashed` while a restart is under way.
- **The render that was drawing** at that moment isn't re-sent, so a page that crashes the engine can't loop through crash and restart. `#recover` records its key (`#lost`) before the lost calls' rejections run. Its page shows an error, which the reader offers Retry for.
- **A restart that didn't reopen the document** (an encrypted one waits for its password) leaves a waiting render nothing to draw from, so it rejects with an `AbortError`. The views already treat that as a cancelled render, so nothing is marked failed or reported.

`stop()` takes the same path. The reader's Stop and the hard cap end the render that hung, and the renders queued behind it are drawn again.

## How it was checked

- **Regression tests in `rpc.browser.test.ts`** ("renders lost with the worker"):
  - a render waiting when the worker crashed is sent again and drawn;
  - after `stop()`, the render that was drawing rejects with `worker-crashed`, and the one waiting is drawn. A test-worker hook, `__holdNextRender`, keeps the worker busy after a render has started, so that render is certainly the one running;
  - a render waiting when an encrypted document's worker crashed rejects with an `AbortError`.
- **Regression tests in the views:** "draws the thumbnails a crash interrupted once the worker restarts, and reports no error" (`navigation.browser.test.ts`) and "draws the pages a crash interrupted once the worker restarts, leaving none in error" (`viewer.browser.test.ts`). Each holds the worker, scrolls so renders queue, crashes the worker, checks that renders were waiting at the crash, and waits until every mounted thumbnail or page is drawn again.
- **Against the old code, all five fail.** With `main`'s client, the crash rejected 13 thumbnail renders, the window got `Uncaught LaminaPdfError: the worker stopped` (the error CI saw), and the interrupted pages stayed in `error`. With the running render's exclusion removed, the `stop()` test fails, since that render is sent again.
- **`@laminapdf/viewer`'s suites:** the browser tests in Chromium and the Node tests pass, as do its typecheck and Biome. CI runs the browser tests in Firefox and WebKit too.

## Left alone

- **Text, links and reading order.** A page's `pageText`, `pageLinks` and `readingOrder` calls go through `call`, not `render`, and aren't sent again. A page interrupted while they waited is drawn again but has no text layer, links or paragraph order until it is mounted again. Re-sending other calls safely needs each to be marked as one that can be repeated; that is a separate change.
- **Find** (`find/controller.ts`) passes a failed call to `reportError`, so a search running when the worker crashes stops where it was and reports the error.
- **The views' cache keys.** A retried render's bitmap is keyed by the document ID read before the restart. Once `restarted` updates the ID, that page renders once more. It costs one render per interrupted page and shows nothing wrong.
