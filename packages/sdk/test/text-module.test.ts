// With the text module loaded, the extract module isn't (the text module extracts too), and
// documents extract text from the text module (B9c). Its own file: a fresh module instance.
import { Builder, Document, Font, init, isExtractLoaded, isLaminaPdfError, reset, wasmMemory } from "@laminapdf/sdk";
import { beforeAll, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true, extract: true }));

it("skips the extract module when the text module is loaded, and still extracts", () => {
  expect([isExtractLoaded(), "extract" in wasmMemory(), "text" in wasmMemory()]).toEqual([true, false, true]);
  expect(Document.open(read("fixtures/golden/examples/hello.pdf")).page(0).text().text()).toBe("Hello, world!");
});

it("refuses a builder's font from before reset() with instance-reset", () => {
  const builder = new Builder();
  const helvetica = Font.standard("Helvetica");
  expect(builder.font(helvetica).postscriptName).toBe("Helvetica");
  reset();
  expect(
    isLaminaPdfError(
      thrown(() => builder.font(helvetica)),
      "instance-reset",
    ),
  ).toBe(true);
  builder.free();
});
