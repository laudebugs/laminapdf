// The docs site's "Show a PDF" example (examples/show-a-pdf.ts) where the browser has no
// `Symbol.dispose`, as in Safari releases without it (docs/bugs/show-a-pdf-not-disposable-in-safari).
// There TypeScript and esbuild compile `using` into a call to the method under
// `Symbol.for("Symbol.dispose")`. The browsers the tests run in all have `Symbol.dispose` and run
// `using` natively, so this file removes it before it loads the example and disposes of the
// result the way the compiled code does.
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { afterAll, expect, it } from "vitest";
import { read } from "./helpers.js";

const original = Object.getOwnPropertyDescriptor(Symbol, "dispose");
afterAll(() => {
  if (original !== undefined) Object.defineProperty(Symbol, "dispose", original);
});

/**
 * What the code TypeScript and esbuild write for `using` calls at the end of its block: the
 * method under `Symbol.dispose`, or under `Symbol.for("Symbol.dispose")` where that's missing.
 */
function disposeAsCompiled(value: object): void {
  const key = Symbol.dispose ?? Symbol.for("Symbol.dispose");
  const method: unknown = Reflect.get(value, key);
  if (typeof method !== "function") throw new TypeError("Object not disposable");
  method.call(value);
}

it("takes the PDF down through compiled `using` where Symbol.dispose is missing", async () => {
  Reflect.deleteProperty(Symbol, "dispose");
  expect(Symbol.dispose).toBeUndefined();
  // Loaded only now, so the example reads which key to use as it would in such a browser.
  const { showPdf } = await import("../../examples/show-a-pdf.js");
  const container = document.createElement("div");
  Object.assign(container.style, { width: "600px", height: "400px" });
  document.body.append(container);
  const bytes = new Uint8Array(await read("fixtures/golden/examples/hello.pdf"));
  const shown = await showPdf(container, new Blob([bytes], { type: "application/pdf" }), pdfiumWasm);
  expect(shown.pageCount).toBe(1);
  expect(container.childElementCount).toBe(1);
  disposeAsCompiled(shown);
  expect(container.childElementCount).toBe(0);
  container.remove();
});
