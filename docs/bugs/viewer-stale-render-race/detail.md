# A viewer test's stale render could succeed, and its failure failed the whole suite

## What we saw

On laudebugs/laminapdf#719 (run 44131695226, first attempt), `@laminapdf/viewer`'s WebKit suite failed although every test passed. In `rpc.browser.test.ts`, "fails a render queued for a document that another open replaced" failed once and passed on its retry, and Vitest then reported one unhandled rejection, `Error: expected a rejection` from `test/browser/helpers.ts:104`. An unhandled error fails the run, so the retry, which should have made this a flaky pass (ADR 0018 §4), still failed the job and `fast-gate`.

## Cause

Two faults, one in the test and one in its helper.

The test holds the worker with `__busy` for 200 ms, queues a background render of the first document, then opens a second document, so the render should fail as `disposed`. But it read the second file between the two steps: `client.open(await read("fixtures/corpus/fpdf2.pdf"))`. When that read took longer than the hold, as it can in WebKit on a busy runner, the worker drew the render before the open arrived, and the render succeeded.

`rejection()` turns a promise that resolves into one that rejects with "expected a rejection". The test holds that promise while it opens the second file and awaits it only afterwards. Nothing handles it when it rejects, so the browser reports an unhandled rejection, and Vitest counts it against the whole run, whatever the retry does.

Adding a 300 ms pause between the render and the open in `main`'s test reproduces CI's output in WebKit: the test fails on both tries, and Vitest prints the same unhandled rejection.
