# Resolution

The print test (`apps/web/e2e/output.e2e.test.ts`) now waits for the extract module's download to finish before it starts recording requests. The worker always starts that download once the first page draws (`warmExtract`), so the test starts waiting before it opens the file, then waits for `lamina_pdf_extract-….wasm` to finish. The download can no longer land inside the print. The test leaves out no request but the service worker's own and `blob:`/`data:` URLs, so any request made while printing still fails it. The product didn't change: the print makes no request.

Leaving the extract module's files out of the recorded requests also stopped the failure, but it would have hidden a print that started loading the module itself. It would also have needed a new exclusion each time the module's bundling changed.

## Proof

- **The race, forced.** A temporary `page.route` held the extract module's JavaScript chunk for 4 s, which pushes the `.wasm` request past the first page.
  - Without the wait, the test failed in Chromium and Firefox. In Firefox it failed with CI's error: `expected [ Array(1) ] to deeply equal []`, naming `lamina_pdf_extract-….wasm`.
  - With the wait, the same delayed test passed in Chromium, Firefox and WebKit.
- **Without the delay.** The test passes in Chromium, Firefox and WebKit. Each engine reports the worker's download to the page, so the wait ends in each.

## Left for later

Nothing.
