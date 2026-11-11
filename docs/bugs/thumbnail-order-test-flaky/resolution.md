# Resolution: the thumbnail render-order test fails now and then in WebKit on CI

[Diagnosis](detail.md).

## The fix

The test now checks its own setup instead of hoping the hold lasts long enough:

- The worker's `__busy` hook answers with the wall clock (`Date.now()`) when the hold ends.
- The test notes the wall clock just after it sends the visible render.
- The test checks the order only when it sent the render at least 100 ms before the hold ended, so the render queued behind the hold. Otherwise it runs the scenario again with the hold doubled: 1.5 s, then 3 s, then 6 s, each time scrolling the strip further so new thumbnails queue.
- The retry never reruns a wrong order: an attempt whose timing held and whose order is wrong fails at once. It reruns only an attempt whose setup didn't happen, which [bug triage](../../guidelines/bug-triage.md) doesn't count as making a failure pass by retrying.
- If no hold was long enough, it fails saying so: "the worker let go before the visible render was sent, with every hold". A slow runner then shows up as itself, never as a wrong render order.

The test also checks only what it means to check: that no thumbnail rendered before the visible render.

- Before each hold, it waits until the worker has started no render for 300 ms, so the main view has nothing left to queue.
- It reads a thumbnail's scale from a painted thumbnail's canvas (its pixel width over the page's 612 pt).
- It expects a thumbnail among the renders logged, and none before the visible one. A main-view render ahead of it no longer counts against it.

The wait for thumbnails without a canvas never fails the test when it times out: once a hold has ended, thumbnails can paint before the wait sees them, and the timing check catches that case.

The viewer's code is unchanged.

## How it was checked

- **The race, forced**: with the first hold set to 1 ms, that attempt fails the timing check and the retry with a 3 s hold passes, in Chromium, Firefox and WebKit. With every hold at 1 ms, the test fails with its setup message.
- **As written**: the test passes three runs in a row in WebKit, and the navigation and RPC suites (93 tests) pass in all three engines.
- **The second cause**: logging the renders shows thumbnails at 0.196 and the test's render at 0.777, so CI's 0.418 was the main view's. With the quiet wait and the thumbnail check, the navigation suite passes three runs in a row in Chromium, Firefox and WebKit.

## Left for later

- **A hold that waits for a signal.** Holding the worker until the test ends the hold, not for a fixed time, would need a shared memory flag (`Atomics.wait`), which requires cross-origin isolation the viewer doesn't use (ADR 0009).
