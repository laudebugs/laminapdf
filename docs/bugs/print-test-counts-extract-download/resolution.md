# Resolution

The test leaves the extract module's two downloads (`lamina_pdf_extract-….js` and `….wasm`) out of the requests it records (`apps/web/e2e/output.e2e.test.ts`), as it already leaves out the service worker's downloads. Any other request made while it prints still fails it. The product didn't change: the reader should start the download after the first page, and the print makes no request. PR #434 left out only the `.wasm` file; the follow-up adds the `.js` chunk, which the worker requests first and which can race the same way.

## Proof

- **The failure, forced.** A temporary `page.route` that held the extract `.js` chunk for 4 s pushed the `.wasm` request into the print. With the old filter the test failed in Firefox on its run and its retry, with CI's error: `expected [ Array(1) ] to deeply equal []`, naming `lamina_pdf_extract-….wasm`. With the fix, the same delayed test passed 3 of 3 times.
- **Without the delay.** The whole output suite passed in Chromium (9 of 9) and Firefox (9 of 9) with the fix.

## Left for later

Nothing. Waiting for the text layer to load before recording would also work, but the reader shows nothing a test can wait on when the download ends, so the test leaves the files out instead.
