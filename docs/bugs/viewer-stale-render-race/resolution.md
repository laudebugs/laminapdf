# Resolution

The test reads the second file before it holds the worker, so the render and the open reach the worker back to back during the hold (`packages/viewer/test/browser/rpc.browser.test.ts`).

`rejection()` marks the promise it returns as handled when it makes it (`packages/viewer/test/browser/helpers.ts`). A test that awaits it later still fails there with "expected a rejection", but the failure no longer escapes as an unhandled rejection. Five other tests in the same file hold a `rejection()` promise the same way, and the change covers them too.

## Proof

- With the 300 ms pause, `main`'s test fails and Vitest reports the unhandled rejection; the changed test with the same pause fails at its `await` and reports no unhandled error.
- The changed test passes five runs out of five in WebKit, none on a retry, and the viewer's whole WebKit suite passes (198 tests).
