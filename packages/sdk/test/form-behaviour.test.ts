// Form behaviour without JavaScript (todo E8f, ADR 0021) over fixtures/forms/scripts.pdf
// (tools/fixtures/forms.py): what a field's helper calls do, a value checked and shown as the
// field would, calculations after a change, a hidden field, and the script inventory, each saved
// and reopened. A test's title starts with the capability it proves (its tag: capability E.08,
// column pdf), which `cargo xtask capabilities` counts.
import { Document, type FormField, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function open(): Document {
  return Document.open(read("fixtures/forms/scripts.pdf"));
}

function field(doc: Document, name: string): FormField {
  const found = doc.formFields().fields.find((f) => f.name === name);
  if (found === undefined) throw new Error(`no field ${name}`);
  return found;
}

function set(doc: Document, name: string, value: string) {
  return doc.apply([{ op: "setFieldValue", args: { field: field(doc, name).id, value: { type: "text", value } } }]);
}

describe("form behaviour", () => {
  it("[cap E.05 pdf round-trip] keeps a date field's format and value through a save, and shows its dates in it", () => {
    using doc = open();
    set(doc, "due", "1/2/2027");
    using again = Document.open(doc.save({ mode: "incremental" }));
    const due = field(again, "due");
    expect([
      due.value,
      again.fieldBehaviour(due.id).format,
      again.validateField(due.id, "1/2/2027").shown,
    ]).toEqual([{ type: "text", value: "1/2/2027" }, { type: "date", pattern: "mmmm d, yyyy" }, "January 2, 2027"]);
  });

  it("[cap E.07 pdf round-trip] refuses a value outside a field's range, and keeps the range through a save", () => {
    using doc = open();
    const price = field(doc, "price").id;
    expect(doc.validateField(price, "20000").refused).toBe("range");
    using again = Document.open(doc.save({ mode: "incremental" }));
    expect([again.fieldBehaviour(price).validate, again.validateField(price, "99.999").shown]).toEqual([
      { type: "range", min: 0, max: 10000 },
      "$100.00",
    ]);
  });

  it("[cap E.08 pdf round-trip] recalculates the totals after a change and keeps them through a save", () => {
    using doc = open();
    set(doc, "qty", "4");
    using again = Document.open(doc.save({ mode: "incremental" }));
    expect([field(again, "total").value, field(again, "sum").value]).toEqual([
      { type: "text", value: "4938" },
      { type: "text", value: "6176.5" },
    ]);
  });

  it("[cap E.10 pdf round-trip] hides a field and keeps it hidden through a save", () => {
    using doc = open();
    doc.apply([{ op: "setFieldHidden", args: { field: field(doc, "price").id, hidden: true } }]);
    using again = Document.open(doc.save({ mode: "incremental" }));
    // The Hidden flag (2) beside Print (4).
    expect(field(again, "price").widgets[0]?.flags).toBe(6);
  });

  it("brings stale totals up to date on request", () => {
    using doc = open();
    doc.calculate();
    expect(field(doc, "total").value).toEqual({ type: "text", value: "3703.5" });
  });

  it("lists every script with its state and never its text", () => {
    using doc = open();
    const scripts = doc.formScripts();
    const states = scripts.map((s) => s.state);
    expect([
      states.filter((s) => s === "handled").length,
      states.filter((s) => s === "not handled").length,
      scripts.find((s) => s.owner === "document")?.trigger,
    ]).toEqual([28, 10, "named:hello"]);
  });
});
