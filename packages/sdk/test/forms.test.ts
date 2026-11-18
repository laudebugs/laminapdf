// The form: doc.formFields and doc.tabOrder over fixtures/forms/ (tools/fixtures/forms.py), and a
// setFieldValue edit through apply, saved and reopened. A test's title starts with the capability
// it proves (its tag: capability E.06, column pdf), which `cargo xtask capabilities` counts.
import { Document, type FormField, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function open(name: string): Document {
  return Document.open(read(`fixtures/forms/${name}`));
}

function field(doc: Document, name: string): FormField {
  const found = doc.formFields().fields.find((f) => f.name === name);
  if (found === undefined) throw new Error(`no field ${name}`);
  return found;
}

describe("forms", () => {
  it("lists each field with its kind, value and widgets in page space", () => {
    using doc = open("fields.pdf");
    const name = field(doc, "name");
    expect([name.kind, name.value, name.required, name.widgets[0]?.page, name.widgets[0]?.rect]).toEqual([
      "text",
      { type: "text", value: "Ada Lovelace" },
      true,
      0,
      [50, 72, 200, 92],
    ]);
  });

  it("reads an off check box as a null button value", () => {
    using doc = open("fields.pdf");
    doc.apply([
      { op: "setFieldValue", args: { field: field(doc, "agree").id, value: { type: "button", value: null } } },
    ]);
    expect(field(doc, "agree").value).toEqual({ type: "button", value: null });
  });

  it("[cap E.06 pdf round-trip] keeps a read-only field's flags and its new value through a save", () => {
    using doc = open("fields.pdf");
    const first = field(doc, "person.first");
    doc.apply([{ op: "setFieldValue", args: { field: first.id, value: { type: "text", value: "Ada" } } }]);
    using again = Document.open(doc.save({ mode: "incremental" }));
    const reread = field(again, "person.first");
    expect([reread.readOnly, reread.value]).toEqual([true, { type: "text", value: "Ada" }]);
  });

  it("refuses a value that doesn't fit with field-value-invalid, saying which rule", () => {
    using doc = open("fields.pdf");
    const error = thrown(() =>
      doc.apply([
        { op: "setFieldValue", args: { field: field(doc, "name").id, value: { type: "text", value: "x".repeat(41) } } },
      ]),
    );
    expect(isLaminaPdfError(error, "field-value-invalid") && error.data).toEqual({ reason: "max-len" });
  });

  it("[cap E.09 pdf read] gives a page's widgets in row order", () => {
    using doc = open("tabs.pdf");
    const order = doc.tabOrder(0);
    const corners = order.widgets.map((stop) => doc.formFields().fields.find((f) => f.id === stop.field)?.name);
    expect([order.tabs, corners]).toEqual(["row", ["p1-tl", "p1-tr", "p1-bl", "p1-br"]]);
  });
});
