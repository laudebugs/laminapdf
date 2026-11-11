// Document: open, read, errors as LaminaPdfError, edits (transactions, undo), saves.
import { Document, init, isLaminaPdfError, LaminaPdfError } from "@laminapdf/sdk";
import * as wasm from "@laminapdf/wasm/base";
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

  it("throws a LaminaPdfError with every ADR 0026 field for a file that isn't a PDF", () => {
    const e = thrown(() => Document.open(new TextEncoder().encode("not a pdf")));
    expect(e instanceof LaminaPdfError && [e.name, e.code, e.kind, e.docsUrl.endsWith("/not-pdf.md")]).toEqual([
      "LaminaPdfError",
      "not-pdf",
      "input",
      true,
    ]);
  });

  it("surfaces a wrong password as wrong-password", () => {
    const e = thrown(() => Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf"), { password: "nope" }));
    expect(isLaminaPdfError(e, "wrong-password")).toBe(true);
  });

  it("surfaces a missing password as password-required", () => {
    const e = thrown(() => Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf")));
    expect(isLaminaPdfError(e, "password-required")).toBe(true);
  });

  it("normalises the password to NFKC (a decomposed ä opens an R6 file)", () => {
    const doc = Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf"), { password: "Pässwört-6" });
    expect(doc.pageCount).toBeGreaterThan(0);
  });

  it("passes the password unchanged first (R2–R4 passwords are not normalised)", () => {
    const calls: unknown[] = [];
    const original = wasm.WasmDocument.openBytes;
    wasm.WasmDocument.openBytes = (bytes, options) => {
      calls.push(options?.password);
      return original.call(wasm.WasmDocument, bytes, options);
    };
    try {
      thrown(() => Document.open(read("fixtures/encrypted/r6-aes-256-user.pdf"), { password: "ﬁ" }));
    } finally {
      wasm.WasmDocument.openBytes = original;
    }
    expect(calls).toEqual(["ﬁ", "fi"]);
  });

  it("refuses an unknown option and suggests the known one", () => {
    const options = { pasword: "x" } as never;
    const e = thrown(() => Document.open(read("fixtures/golden/examples/hello.pdf"), options));
    expect(isLaminaPdfError(e) && [e.code, e.kind, e.didYouMean]).toEqual(["unknown-option", "usage", ["password"]]);
  });

  it("serialises the error to the ADR 0026 object", () => {
    const e = thrown(() => Document.open(new Uint8Array(0)));
    const json = isLaminaPdfError(e) ? e.toJSON() : undefined;
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
    const undone = doc.undo();
    expect([after, [doc.info.title, doc.pageInfo(0).rotation], undone]).toEqual([
      ["T", 90],
      [undefined, 0],
      { outcome: "applied", touchedPages: [id] },
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
    expect(isLaminaPdfError(e, "nested-transaction")).toBe(true);
  });

  it("refuses an async callback and applies nothing (invalid-argument)", () => {
    const doc = hello();
    const e = thrown(() =>
      doc.transact(async (tx) => {
        tx.setInfo({ title: "async" });
      }),
    );
    expect([isLaminaPdfError(e, "invalid-argument"), doc.hasUnsavedChanges]).toEqual([true, false]);
  });

  it("refuses undo, apply and save inside a transaction of the same document", () => {
    const doc = hello();
    const codes = [() => doc.undo(), () => doc.apply([]), () => doc.save()].map((call) => {
      const e = thrown(() => doc.transact(() => call()));
      return isLaminaPdfError(e) ? e.code : e;
    });
    expect(codes).toEqual(["nested-transaction", "nested-transaction", "nested-transaction"]);
  });

  it("allows a transaction of another document inside a transaction", () => {
    const [a, b] = [hello(), hello()];
    a.transact((tx) => {
      b.transact((inner) => inner.setInfo({ title: "B" }));
      tx.setInfo({ title: "A" });
    });
    expect([a.info.title, b.info.title]).toEqual(["A", "B"]);
  });

  it("refuses ops after the transaction ended", () => {
    const doc = hello();
    let kept: { setInfo(p: object): unknown } | undefined;
    doc.transact((tx) => {
      kept = tx;
    });
    expect(
      isLaminaPdfError(
        thrown(() => kept?.setInfo({})),
        "no-transaction",
      ),
    ).toBe(true);
  });

  it("redoes an undone edit", () => {
    const doc = hello();
    doc.transact((tx) => tx.setInfo({ subject: "S" }));
    doc.undo();
    expect([doc.redo(), doc.info.subject]).toEqual([{ outcome: "applied", touchedPages: [] }, "S"]);
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
