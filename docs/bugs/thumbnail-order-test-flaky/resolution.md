# Resolution: the thumbnail render-order test fails now and then in WebKit on CI

[Diagnosis](detail.md).

## The fix

The test now checks its own setup instead of hoping the hold lasts long enough:

- The test worker's `__busy` hook answers with the wall clock (`Date.now()`) when it lets go.
- The test notes the wall clock just after it sends the visible render.
- The order is checked only when the render was sent at least 100 ms before the worker let go, so it was queued behind the hold. Otherwise the test runs the scenario again with the hold doubled: 1.5 s, then 3 s, then 6 s, each time scrolling the strip further so new thumbnails queue.
- If no hold was long enough, it fails saying so: "the worker let go before the visible render was sent, with every hold". A slow runner then shows up as itself, never as a wrong render order.

The wait for unpainted thumbnails no longer fails the test: once a hold is over, thumbnails can paint before the wait sees them, and the timing check catches that case.

The viewer's code is unchanged.

## How it was checked

- **The race, forced**: with the first hold set to 1 ms, the first attempt is set aside and the 3 s one passes, in Chromium, Firefox and WebKit. With every hold at 1 ms, the test fails with its setup message.
- **As written**: three runs in WebKit and the navigation and RPC suites in the three engines (93 tests) pass.

## Left for later

- **A hold that waits for a signal.** Holding the worker until the test releases it, not for a time, would need a shared memory flag (`Atomics.wait`), which needs cross-origin isolation the viewer doesn't use (ADR 0009).
