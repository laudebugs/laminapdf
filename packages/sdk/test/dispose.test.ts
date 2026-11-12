// Disposal: every handle on WebAssembly memory frees with `free()` or `using`, a second free does
// nothing, and every call afterwards throws `disposed`, on the handle and on what it made.
import { Builder, Document, Image, init, isLaminaPdfError, Page, PageRef, SaveJob } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

/** Every exported class that holds WebAssembly memory. */
const INVENTORY = { Builder, Document, Image, Page, PageRef, SaveJob };

const hello = (): Document => Document.open(read("fixtures/golden/examples/hello.pdf"));
const red = (): Image => Image.rgba(1, 1, new Uint8Array([255, 0, 0, 255]));

/** Whether `f` throws `disposed`, and not a glue error such as "null pointer passed to rust". */
const disposed = (f: () => unknown): boolean => isLaminaPdfError(thrown(f), "disposed");

/** One call of each `Document` member. */
const DOCUMENT_CALLS: readonly [string, (doc: Document) => unknown][] = [
  ["pageCount", (d) => d.pageCount],
  ["pages", (d) => d.pages],
  ["pageInfo", (d) => d.pageInfo(0)],
  ["page", (d) => d.page(0)],
  ["pageById", (d) => d.pageById("p3.0")],
  ["info", (d) => d.info],
  ["warnings", (d) => d.warnings],
  ["hasUnsavedChanges", (d) => d.hasUnsavedChanges],
  ["canExtractText", (d) => d.canExtractText],
  ["apply", (d) => d.apply([])],
  ["applyLog", (d) => d.applyLog({ v: 1, ops: [] })],
  ["transact", (d) => d.transact(() => undefined)],
  ["undo", (d) => d.undo()],
  ["redo", (d) => d.redo()],
  ["save", (d) => d.save()],
  ["saveWithReport", (d) => d.saveWithReport()],
  ["saveCopy", (d) => d.saveCopy()],
  ["saveJob", (d) => d.saveJob()],
  ["rebase", (d) => d.rebase(new Uint8Array())],
];

/** One call of each `PageRef` member. */
const PAGE_CALLS: readonly [string, (page: PageRef) => unknown][] = [
  ["id", (p) => p.id],
  ["index", (p) => p.index],
  ["info", (p) => p.info],
  ["text", (p) => p.text()],
];

beforeAll(() => init());

describe("the handles", () => {
  it.each(Object.entries(INVENTORY))("%s frees with free() and Symbol.dispose", (_, handle) => {
    const prototype = handle.prototype as unknown as Record<PropertyKey, unknown>;
    expect([typeof prototype.free, typeof prototype[Symbol.dispose]]).toEqual(["function", "function"]);
  });

  it("frees a document at the end of its using block", () => {
    let held: Document | undefined;
    {
      using doc = hello();
      held = doc;
      expect(doc.pageCount).toBe(1);
    }
    expect(disposed(() => held?.pageCount)).toBe(true);
  });

  it("does nothing on a second free, and on dispose after free", () => {
    const doc = hello();
    doc.free();
    doc.free();
    doc[Symbol.dispose]();
    const builder = new Builder();
    builder.free();
    builder.free();
    const image = red();
    image.free();
    image.free();
    expect(disposed(() => doc.pageCount)).toBe(true);
  });
});

describe("a freed document", () => {
  it.each(DOCUMENT_CALLS)("throws disposed from %s", (_, call) => {
    const doc = hello();
    doc.free();
    expect(disposed(() => call(doc))).toBe(true);
  });

  it("throws disposed from ensure", async () => {
    const doc = hello();
    doc.free();
    await expect(doc.ensure(() => 1)).rejects.toSatisfy((e: unknown) => isLaminaPdfError(e, "disposed"));
  });

  it.each(PAGE_CALLS)("frees its pages: %s throws disposed", (_, call) => {
    const doc = hello();
    const page = doc.page(0);
    doc.free();
    expect(disposed(() => call(page))).toBe(true);
  });

  it("frees its save jobs: step and cancel throw disposed", () => {
    const doc = hello();
    const job: SaveJob = doc.saveJob();
    doc.free();
    expect([disposed(() => job.step(1)), disposed(() => job.cancel())]).toEqual([true, true]);
  });

  it("leaves a page freed on its own disposed, and its document working", () => {
    const doc = hello();
    const page = doc.page(0);
    page.free();
    expect([disposed(() => page.info), doc.page(0).index]).toEqual([true, 0]);
    doc.free();
  });
});

describe("a freed builder", () => {
  it("throws disposed from its methods and its pages'", () => {
    const builder = new Builder();
    const page = builder.page("A6");
    const surface = page.surface();
    builder.free();
    expect([
      disposed(() => builder.page("A6")),
      disposed(() => builder.metadata({ title: "x" })),
      disposed(() => builder.finish()),
      disposed(() => page.rotate(90)),
      disposed(() => surface.save()),
      disposed(() => surface.path((p) => p.rect([0, 0, 1, 1])).fill()),
      disposed(() => page.finish()),
    ]).toEqual([true, true, true, true, true, true, true]);
  });

  it("drops a page freed on its own, and builds the others", () => {
    const builder = new Builder();
    const dropped = builder.page("A6");
    builder.page("A6").finish();
    dropped.free();
    using doc = builder.finish();
    expect([disposed(() => dropped.surface().save()), doc.pageCount]).toEqual([true, 1]);
  });
});

describe("a freed image", () => {
  it("throws disposed when drawn", () => {
    const image = red();
    expect(image.width).toBe(1);
    image.free();
    const builder = new Builder();
    const surface = builder.page("A6").surface();
    expect(disposed(() => surface.image(image, [0, 0, 10, 10]))).toBe(true);
    builder.free();
  });
});
