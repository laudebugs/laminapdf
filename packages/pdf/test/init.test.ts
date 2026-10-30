// init() under the `node` condition: nothing works before it, every input form loads, it is
// idempotent, and the text module is opt-in.
import {
  Builder,
  Document,
  Font,
  init,
  isInitialized,
  isLaminaError,
  isTextLoaded,
  loadText,
  wasmMemory,
} from "@lamina/pdf";
import { describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

describe("init", () => {
  it("is required before the first call (not-initialized)", () => {
    const e = thrown(() => Document.open(read("fixtures/golden/examples/hello.pdf")));
    expect(isLaminaError(e, "not-initialized") && e.kind).toBe("state");
  });

  it("loads the base module from the file next to the package", async () => {
    await init();
    expect([isInitialized(), isTextLoaded()]).toEqual([true, false]);
  });

  it("is idempotent", async () => {
    await init();
    await init(read("packages/wasm/dist/lamina_base.wasm"));
    expect(Document.open(read("fixtures/golden/examples/hello.pdf")).pageCount).toBe(1);
  });

  it("builds without text in the base module", () => {
    const b = new Builder();
    b.page("A6").finish();
    expect([b.hasText, b.finish().pageCount]).toEqual([false, 1]);
  });

  it("refuses text in a base-module builder (text-module-not-loaded)", () => {
    const page = new Builder().page("A6");
    const e = thrown(() => page.surface().text("x", { font: Font.standard("Helvetica"), size: 9, at: [9, 9] }));
    expect(isLaminaError(e, "text-module-not-loaded")).toBe(true);
  });

  it("refuses new Builder({ text: true }) before the text module is loaded", () => {
    expect(
      isLaminaError(
        thrown(() => new Builder({ text: true })),
        "text-module-not-loaded",
      ),
    ).toBe(true);
  });

  it("loads the text module from a compiled WebAssembly.Module", async () => {
    await loadText(await WebAssembly.compile(read("packages/wasm/dist/lamina_text.wasm").slice()));
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
