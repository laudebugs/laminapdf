// init() under the `node` condition: nothing works before it, every input form loads, it is
// idempotent, and the text module is opt-in.
import {
  Builder,
  Document,
  Font,
  init,
  isInitialized,
  isLaminaPdfError,
  isTextLoaded,
  loadText,
  wasmMemory,
} from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

describe("init", () => {
  it("is required before the first call (not-initialized)", () => {
    const e = thrown(() => Document.open(read("fixtures/golden/examples/hello.pdf")));
    expect(isLaminaPdfError(e, "not-initialized") && e.kind).toBe("state");
  });

  it("loads the base module from the file next to the package", async () => {
    await init();
    expect([isInitialized(), isTextLoaded()]).toEqual([true, false]);
  });

  it("is idempotent", async () => {
    await init();
    await init({});
    expect(Document.open(read("fixtures/golden/examples/hello.pdf")).pageCount).toBe(1);
  });

  it("refuses a different source once the module is loaded (already-initialized)", async () => {
    const e = await init(read("packages/wasm/dist/lamina_pdf_base.wasm")).catch((error: unknown) => error);
    expect(isLaminaPdfError(e, "already-initialized")).toBe(true);
  });

  it("builds without text in the base module", () => {
    const b = new Builder();
    b.page("A6").finish();
    expect([b.hasText, b.finish().pageCount]).toEqual([false, 1]);
  });

  it("refuses text in a base-module builder (text-module-not-loaded)", () => {
    const page = new Builder().page("A6");
    const e = thrown(() => page.surface().text("x", { font: Font.standard("Helvetica"), size: 9, at: [9, 9] }));
    expect(isLaminaPdfError(e, "text-module-not-loaded")).toBe(true);
  });

  it("refuses new Builder({ text: true }) before the text module is loaded", () => {
    expect(
      isLaminaPdfError(
        thrown(() => new Builder({ text: true })),
        "text-module-not-loaded",
      ),
    ).toBe(true);
  });

  it("loads the text module from a compiled WebAssembly.Module", async () => {
    const module = await WebAssembly.compile(read("packages/wasm/dist/lamina_pdf_text.wasm").slice());
    await loadText(module);
    await loadText(module); // the same source again is fine
    const b = new Builder();
    const page = b.page("A6");
    page.surface().text("Hi", { font: Font.standard("Helvetica"), size: 9, at: [9, 20] });
    page.finish();
    expect([b.hasText, b.finish().pageCount]).toEqual([true, 1]);
  });

  it("reports each loaded module's memory", () => {
    const memory = wasmMemory();
    expect([typeof memory.base, typeof memory.text]).toEqual(["number", "number"]);
  });
});
