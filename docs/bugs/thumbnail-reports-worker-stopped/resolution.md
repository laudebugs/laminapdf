# Resolution: a thumbnail reports "the worker stopped" when the worker crashes mid-draw

[Diagnosis](detail.md).

## The fix

The thumbnail strip no longer passes a `worker-crashed` rejection to `reportError`. The thumbnail keeps its placeholder and isn't asked for again while it stays mounted, as ADR 0009 §3.6 asks for the render that was running. Other render failures are still reported.

The check follows how the viewer already splits the work: the client decides which render a lost worker takes down, and each view decides what to show for it. The main view shows the page in error, with Retry. The strip shows its placeholder. Neither reports the loss as its own failure, because the client's events already report it. `terminate()` rejects the same way and fires no event, since the app ended the worker itself.

## How it was checked

- **A regression test in `navigation.browser.test.ts`:** "reports no error for the thumbnail that was drawing when the worker stopped". It waits until the main view is drawn, holds the worker on the next render with `__holdNextRender`, checks from its scale that this is the strip's thumbnail, then stops the worker. It reproduces the nightly's failure every time: instead of racing the crash, the thumbnail's render is held until the worker stops. It checks that the restart reopened the document, that the thumbnail kept its placeholder, and that the window got no error.
- **Against the old strip, the test fails** with `Uncaught LaminaPdfError: the worker stopped`, the error the nightly saw.
- **`@laminapdf/viewer`'s suites:** the 137 browser tests in Chromium, with no unhandled error, and the 367 Node tests pass, as do its typecheck and Biome. CI runs the browser tests in Firefox and WebKit too.

## Left alone

- **Find** still passes a failed call to `reportError`, so a search running when the worker crashes reports the crash. The same change would fit there, in its own PR.
- **The interrupted thumbnail stays blank** until it is scrolled out of view and back. The ADR wants the running request left alone so a page that crashes the engine can't loop; a Retry for thumbnails would be a separate change.
