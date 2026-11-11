# The print test counted the text layer's download as the print's

## What happened

The reader's end-to-end test "print, download and properties in firefox › prints 50 portrait and landscape pages, each loaded at its own size, with no CSP violation or request" failed on its run and its retry in CI (PR #411, run 43064138898, 2026-11-11):

```
AssertionError: expected [ Array(1) ] to deeply equal []
+ [
+   "https://127.0.0.1:38599/assets/lamina_pdf_extract-EmQat9AN.wasm",
+ ]
```

The same test had failed in Firefox on that PR before, and passed when the job ran again. PR #411 doesn't change the reader, so the fault came from `main`.

A user could not see the fault. The print made no request. The one request the test caught was the extract module, which the reader downloads once for the text layer.

## Cause

The test opens a file, waits for the first page's canvas, then starts recording the page's requests and prints. The document worker starts downloading the extract module right after it draws the first page (`warmExtract` in `packages/viewer/src/worker/document.ts`), so that download doesn't compete with the first page. On a fast run the request goes out before the test starts recording. On a slow run, as in Firefox on a busy CI runner, it goes out after the recording starts, and the test counts it as the print's.
