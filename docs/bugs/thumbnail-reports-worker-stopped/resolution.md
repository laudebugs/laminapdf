# Resolution: a thumbnail reports "the worker stopped" when the worker crashes mid-draw

[Diagnosis](detail.md).

## The fix

The thumbnail strip no longer passes a `worker-crashed` rejection to `reportError`. The thumbnail keeps its placeholder and isn't asked for again while it stays mounted, as ADR 0009 §3.6 asks for the render that was running. Other render failures are still reported.

## How it was checked

- **A regression test in `navigation.browser.test.ts`:** "reports no error for the thumbnail that was drawing when the worker stopped". It waits until the main view is drawn, holds the worker on the next render (the strip's first thumbnail) with `__holdNextRender`, then stops the worker. It checks that the restart reopened the document, that the thumbnail kept its placeholder, and that the window got no error. Without the fix, the window gets `the worker stopped`.
- **`@laminapdf/viewer`'s browser tests** in Chromium; CI runs them in Firefox and WebKit too.

## Left alone

- **Find** still passes a failed call to `reportError`, so a search running when the worker crashes reports the crash. The same change would fit there, in its own PR.
- **The interrupted thumbnail stays blank** until it is scrolled out of view and back. The ADR wants the running request left alone so a page that crashes the engine can't loop; a Retry for thumbnails would be a separate change.
