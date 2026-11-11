# Resolution

The test leaves the extract module's download out of the requests it records (`apps/web/e2e/output.e2e.test.ts`), as it already leaves out the service worker's downloads. The test still fails on any other request made while it prints. The product didn't change: the reader should start the download after the first page, and the print makes no request.

## Proof

- **The cause, from the log.** The failing run's only extra request is `lamina_pdf_extract-….wasm`, the file `warmExtract` loads. The print frame loads its pages as `blob:` URLs, which the test already leaves out.
- **CI.** The test passes in Chromium, Firefox and WebKit on this PR's run.

The race depends on the runner's speed, so no local test fails without the fix.
