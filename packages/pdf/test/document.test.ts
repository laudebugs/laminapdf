// Document: open, read, errors as LaminaError, edits (transactions, undo), saves.
import { Document, init, isLaminaError, LaminaError } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

const hello = (): Document => Document.open(read("fixtures/golden/examples/hello.pdf"));

beforeAll(() => init());

describe("Document.open", () => {
  it("reads the pages and the Info dictionary", () => {
    const doc = hello();
    expect([doc.pageCount, doc.pages.length, Math.round(doc.pageInfo(0).width)]).toEqual([1, 1, 595]);
  });

  it("accepts an ArrayBuffer", () => {
    const bytes = read("fixtures/golden/examples/hello.pdf");
    expect(Document.open(bytes.slice().buffer).pageCount).toBe(1);
  });

  it("throws a LaminaError with every ADR 0026 field for a file that isn't a PDF", () => {
    const e = thrown(() => Document.open(new TextEncoder().encode("not a pdf")));
    expect(e instanceof LaminaError && [e.name, e.code, e.kind, e.docsUrl.endsWith("/not-pdf.md")]).toEqual([
      "LaminaError",
      "not-pdf",
      "input",
      true,
    ]);
  });

  it("surfaces a wrong password as wrong-password", () => {
    const e = thrown(() => Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf"), { password: "nope" }));
    expect(isLaminaError(e, "wrong-password")).toBe(true);
  });

  it("surfaces a missing password as password-required", () => {
    const e = thrown(() => Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf")));
    expect(isLaminaError(e, "password-required")).toBe(true);
  });

  it("normalises the password to NFKC (a decomposed ä opens an R6 file)", () => {
    const doc = Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf"), { password: "Pässwört-6" });
    expect(doc.pageCount).toBeGreaterThan(0);
  });

  it("refuses an unknown option and suggests the known one", () => {
    const options = { pasword: "x" } as never;
    const e = thrown(() => Document.open(read("fixtures/golden/examples/hello.pdf"), options));
    expect(isLaminaError(e) && [e.code, e.kind, e.didYouMean]).toEqual(["unknown-option", "usage", ["password"]]);
  });

  it("serialises the error to the ADR 0026 object", () => {
    const e = thrown(() => Document.open(new Uint8Array(0)));
    const json = isLaminaError(e) ? e.toJSON() : undefined;
    expect(Object.keys(json ?? {}).filter((k) => !["hint", "location", "data"].includes(k))).toEqual([
      "code",
      "kind",
      "message",
      "docsUrl",
    ]);
  });
});

describe("Document edits", () => {
  it("applies a transaction and undoes it", () => {
    const doc = hello();
    const id = doc.page(0).id;
    doc.transact((tx) => tx.setInfo({ title: "T" }).rotatePage(id, 90));
    const after = [doc.info.title, doc.pageInfo(0).rotation];
    doc.undo();
    expect([after, [doc.info.title, doc.pageInfo(0).rotation]]).toEqual([
      ["T", 90],
      [undefined, 0],
    ]);
  });

  it("applies nothing when the transaction's callback throws", () => {
    const doc = hello();
    expect(() =>
      doc.transact((tx) => {
        tx.setInfo({ title: "never" });
        throw new Error("changed my mind");
      }),
    ).toThrow("changed my mind");
    expect(doc.hasUnsavedChanges).toBe(false);
  });

  it("refuses a nested transaction", () => {
    const doc = hello();
    const e = thrown(() => doc.transact(() => doc.transact(() => undefined)));
    expect(isLaminaError(e, "nested-transaction")).toBe(true);
  });

  it("refuses ops after the transaction ended", () => {
    const doc = hello();
    let kept: { setInfo(p: object): unknown } | undefined;
    doc.transact((tx) => {
      kept = tx;
    });
    expect(
      isLaminaError(
        thrown(() => kept?.setInfo({})),
        "no-transaction",
      ),
    ).toBe(true);
  });

  it("redoes an undone edit", () => {
    const doc = hello();
    doc.transact((tx) => tx.setInfo({ subject: "S" }));
    doc.undo();
    expect([doc.redo(), doc.info.subject]).toEqual(["applied", "S"]);
  });
});

describe("Document.save", () => {
  it("saves incrementally, appending to the original bytes", () => {
    const original = read("fixtures/golden/examples/hello.pdf");
    const doc = Document.open(original);
    doc.transact((tx) => tx.setInfo({ title: "Saved" }));
    const { bytes, report } = doc.saveWithReport({ mode: "incremental" });
    expect([report.mode, original.every((b, i) => bytes[i] === b)]).toEqual(["incremental", true]);
  });

  it("steps a save job to the end", () => {
    const job = hello().saveJob({ mode: "fullRewrite" });
    let step = job.step(1);
    while (step.state === "progress") step = job.step(1);
    expect(Document.open(step.bytes).pageCount).toBe(1);
  });

  it("frees a document twice without failing", () => {
    const doc = hello();
    doc.free();
    expect(() => doc.free()).not.toThrow();
  });

  it("frees a document at the end of a using block", () => {
    let warnings = -1;
    {
      using doc = hello();
      warnings = doc.warnings.length;
    }
    expect(warnings).toBe(0);
  });
});
