// Form data (todo E8g) over fixtures/forms/fields.pdf (tools/fixtures/forms.py): filled from an
// object, from each file format and back to its defaults, and flattened (todo E8p), each as one
// edit, saved and reopened. A
// test's title starts with the capability it proves (its tag: capability E.12, column pdf), which
// `cargo xtask capabilities` counts.
import { Document, type FormField, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function open(): Document {
  return Document.open(read("fixtures/forms/fields.pdf"));
}

function field(doc: Document, name: string): FormField {
  const found = doc.formFields().fields.find((f) => f.name === name);
  if (found === undefined) throw new Error(`no field ${name}`);
  return found;
}

describe("form data", () => {
  it("[cap E.12 pdf round-trip] fills a form from an object as one edit, and keeps the answers through a save", () => {
    using doc = open();
    const report = doc.fillForm({
      fields: { name: "Grace Hopper", agree: false, toppings: ["Ham"], phone: "555 0100", signature: "x" },
    });
    expect([report.unknown, report.refused]).toEqual([["phone"], [{ name: "signature", code: "field-is-signature" }]]);
    using again = Document.open(doc.save({ mode: "incremental" }));
    expect([field(again, "name").value, field(again, "agree").value, field(again, "toppings").value]).toEqual([
      { type: "text", value: "Grace Hopper" },
      { type: "button", value: null },
      { type: "choice", value: ["Ham"] },
    ]);
    doc.undo();
    expect(field(doc, "name").value).toEqual({ type: "text", value: "Ada Lovelace" });
  });

  it("[cap E.11 pdf round-trip] exports each format and imports it into an emptied copy, which exports the same file and keeps the answers through a save", () => {
    const text = (bytes: Uint8Array) => new TextDecoder().decode(bytes);
    using original = open();
    for (const format of ["json", "xfdf", "fdf"] as const) {
      const exported = original.exportFormData(format, "fields.pdf");
      using copy = open();
      copy.resetFields();
      const report = copy.importFormData(exported);
      expect([format, report.unknown, report.refused, text(copy.exportFormData(format, "fields.pdf"))]).toEqual([
        format,
        [],
        [],
        text(exported),
      ]);
      // A save gives the file a new changing /ID, which XFDF and FDF record, so JSON compares after it.
      using again = Document.open(copy.save({ mode: "incremental" }));
      expect(text(again.exportFormData("json"))).toBe(text(original.exportFormData("json")));
    }
  });

  it("[cap E.13 pdf round-trip] resets the fields named to their defaults, and keeps them through a save", () => {
    using doc = open();
    const name = field(doc, "name").id;
    doc.resetFields([name]);
    using again = Document.open(doc.save({ mode: "incremental" }));
    expect([field(again, "name").value, field(again, "address").value]).toEqual([
      undefined,
      { type: "text", value: "12 Main Street" },
    ]);
  });

  it("[cap E.14 pdf round-trip] flattens every field but signatures into the pages, as one edit, and keeps the result through a save", () => {
    using doc = open();
    const edited = doc.flattenForm(undefined, "view");
    doc.undo();
    const restored = field(doc, "name").value;
    doc.redo();
    using again = Document.open(doc.save({ mode: "incremental" }));
    const left = again
      .formFields()
      .fields.map((f) => f.name)
      .sort();
    expect([restored, left, (edited.warnings ?? []).map((w) => w.code)]).toEqual([
      { type: "text", value: "Ada Lovelace" },
      ["signature", "submit"],
      ["flatten-no-appearance"],
    ]);
  });

  it("refuses an XFDF file that declares a DTD", () => {
    using doc = open();
    expect(() => doc.importFormData(read("fixtures/forms/data/dtd.xfdf"))).toThrow(
      expect.objectContaining({ code: "interchange-dtd" }),
    );
  });
});
