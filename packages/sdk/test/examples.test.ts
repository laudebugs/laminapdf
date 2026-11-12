// The TS examples produce the Rust examples' bytes (fixtures/golden/examples/, ADR 0018 §5) in
// Node: the shared list (test/suite/examples.ts) every runtime of the B6c matrix runs. The
// report's hash and the performance smoke are test/node/measure.test.ts.
import { Builder, Document, Font, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { openUntrusted } from "../examples/open-untrusted.js";
import { pageNumbers } from "../examples/page-numbers.js";
import { hello as nodeQuickStart } from "../examples/quick-start/node.js";
import { openOrAsk } from "../examples/react-to-error-codes.js";
import { readDocument } from "../examples/read-document.js";
import { rotateLandscapePages } from "../examples/rotate-pages.js";
import { read } from "./helpers.js";
import { registerQuickStart, runSuite } from "./suite/examples.js";

beforeAll(() => init({ text: true }));

describe("examples", () => {
  runSuite("node", read, it, "examples");
  registerQuickStart("node", read, it, nodeQuickStart);

  it("uses one PDF font for equal fonts created per call", () => {
    const build = (font: () => Font) => {
      const b = new Builder({ id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" });
      const page = b.page("A6");
      page.surface().text("one", { font: font(), size: 12, at: [10, 20] });
      page.surface().text("two", { font: font(), size: 12, at: [10, 40] });
      const noto = Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
      page.surface().text("three", { font: noto, size: 12, at: [10, 60] });
      page
        .surface()
        .text("four", { font: Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf")), size: 12, at: [10, 80] });
      page.finish();
      return b.finish().save();
    };
    const helvetica = Font.standard("Helvetica");
    expect(build(() => Font.standard("Helvetica"))).toEqual(build(() => helvetica));
  });

  it("opens an encrypted upload with its password and reports a missing or wrong one by code", () => {
    const file = read("fixtures/encrypted/r6-aes-256-user.pdf");
    expect(openUntrusted(file, "Pässwört-6")).toEqual({ ok: true, pages: 2, warnings: [] });
    expect(openUntrusted(file)).toMatchObject({ ok: false, code: "password-required" });
    expect(openUntrusted(file, "nope")).toMatchObject({ ok: false, code: "wrong-password" });
    expect(openUntrusted(new TextEncoder().encode("not a PDF"))).toMatchObject({ ok: false, code: "not-pdf" });
  });

  it("turns a missing or wrong password and a non-PDF into the app's next step", () => {
    const file = read("fixtures/encrypted/r6-aes-256-user.pdf");
    expect(openOrAsk(file)).toEqual({ step: "ask-for-password" });
    expect(openOrAsk(file, "nope")).toEqual({ step: "ask-again" });
    expect(openOrAsk(file, "Pässwört-6")).toEqual({ step: "show", pages: 2, warnings: [] });
    expect(openOrAsk(new TextEncoder().encode("not a PDF"))).toEqual({ step: "not-a-pdf" });
  });

  it("reads a file's pages, metadata, first page's text and warnings", () => {
    const summary = readDocument(read("fixtures/golden/examples/generate_then_edit.pdf"));
    expect(summary).toMatchObject({ title: "Final report", authors: ["Ada Lovelace"], warnings: [] });
    expect(summary.pages).toHaveLength(1);
    expect(summary.pages[0]?.width).toBeCloseTo(595.28, 1);
    expect(summary.firstPageText).toBe("Draft report");
  });

  it("turns only the landscape pages, in a revision appended to the original bytes", () => {
    const b = new Builder({ id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" });
    b.page("A4").finish();
    b.page([842, 595]).finish();
    using built = b.finish();
    const original = built.save();
    const rotated = rotateLandscapePages(original);
    expect(rotated.subarray(0, original.length)).toEqual(original);
    using doc = Document.open(rotated);
    expect(doc.pages.map((p) => p.rotation)).toEqual([0, 90]);
    expect(doc.pages.every((p) => p.height >= p.width)).toBe(true);
  });

  it("numbers each page out of the total", () => {
    const options = { id: "000102030405060708090a0b0c0d0e0f", creationDate: "D:20261030120000Z" };
    const bytes = pageNumbers(["Summary", "Figures", "Appendix"], options);
    expect(pageNumbers(["Summary", "Figures", "Appendix"], options)).toEqual(bytes);
    using doc = Document.open(bytes);
    expect(doc.pageCount).toBe(3);
    expect(doc.metadata().title).toBe("Summary");
    using second = doc.page(1);
    expect(second.text().lines).toEqual(["Figures", "Page 2 of 3"]);
  });
});
