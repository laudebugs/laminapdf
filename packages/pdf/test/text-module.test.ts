// With the text module loaded, the extract module isn't (the text module extracts too), and
// documents extract text from the text module (B9c). Its own file: a fresh module instance.
import { Document, init, isExtractLoaded, wasmMemory } from "@laminapdf/pdf";
import { beforeAll, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init({ text: true, extract: true }));

it("skips the extract module when the text module is loaded, and still extracts", () => {
  expect([isExtractLoaded(), "extract" in wasmMemory(), "text" in wasmMemory()]).toEqual([true, false, true]);
  expect(Document.open(read("fixtures/golden/examples/hello.pdf")).page(0).text().text()).toBe("Hello, world!");
});
