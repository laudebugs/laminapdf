# The "Show a PDF" example threw "not disposable" in Safari

## What happened

The docs site's guide [Show a PDF in a web page](../../../apps/docs/src/content/docs/guides/show-a-pdf.mdx) tells readers to write `using shown = await showPdf(container, file, pdfiumWasm)`. In a Safari release that has no `Symbol.dispose`, that line throws a `TypeError` saying the object isn't disposable, once TypeScript or esbuild has compiled the `using` declaration for that browser. The PDF still showed, but the guide's own code failed, and the viewer and its Web Worker stayed up.

The example's browser test passed in WebKit all the same, so nothing caught the fault.

## How to reproduce it

Remove `Symbol.dispose` before the example loads, then dispose of what `showPdf` returns the way compiled `using` does:

```ts
Reflect.deleteProperty(Symbol, "dispose");
const { showPdf } = await import("./examples/show-a-pdf.js");
const shown = await showPdf(container, file, pdfiumWasm);
const key = Symbol.dispose ?? Symbol.for("Symbol.dispose"); // what the compiled code looks up
shown[key](); // TypeError: shown[key] is not a function
```

`packages/viewer/test/browser/show-a-pdf-without-symbol-dispose.browser.test.ts` runs these steps in Chromium, Firefox and WebKit. Against the example before the fix it fails in all three.

## Cause

`packages/viewer/examples/show-a-pdf.ts` returned `{ pageCount, [Symbol.dispose]: close }`. Where `Symbol.dispose` is `undefined`, the computed key becomes the string `"undefined"`, so the close method sat under `"undefined"`. The code TypeScript and esbuild write for `using` looks for the method under `Symbol.dispose`, or under `Symbol.for("Symbol.dispose")` where the former is missing, and found nothing there. `@laminapdf/sdk` already handles this case: `packages/sdk/src/internal/handle.ts` puts its handles' dispose method under `Symbol.for("Symbol.dispose")` where `Symbol.dispose` is missing.

## Why the tests missed it

Playwright 1.64's WebKit reports itself as Safari 27.2 and has both `Symbol.dispose` and native `using`, as do the Chromium and Firefox the tests run, and Vite 8 leaves `using` as written. So no browser in the suite ever took the missing-symbol path. Had it done so, the old test would still have passed: it called `shown[Symbol.dispose]()`, which reads the same `"undefined"` key the example wrote.
