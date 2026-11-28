// The XFA part: doc.xfa over fixtures/forms/xfa-static.pdf and xfa-dynamic.pdf
// (tools/fixtures/forms.py) and the removeXfa edit through apply. A test's title starts with the capability it proves (its tag), which
// `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function open(name: string): Document {
  return Document.open(read(`fixtures/forms/${name}`));
}

const text = (bytes: Uint8Array): string => new TextDecoder().decode(bytes);

describe("xfa", () => {
  it("[cap E.17 pdf read] names a static form and gives its packets as bytes", () => {
    using doc = open("xfa-static.pdf");
    const xfa = doc.xfa();
    expect([xfa?.kind, xfa?.signals, xfa?.packets.map((p) => p.name)]).toEqual([
      "static",
      [],
      ["template", "datasets"],
    ]);
    expect(text(xfa?.packets[0]?.bytes ?? new Uint8Array())).toContain('<field name="name"/>');
  });

  it("[cap E.17 pdf read] names a dynamic form and what made it dynamic", () => {
    using doc = open("xfa-dynamic.pdf");
    const xfa = doc.xfa();
    expect([xfa?.kind, xfa?.signals, xfa?.packets.map((p) => p.name)]).toEqual([
      "dynamic",
      ["needsRendering", "dynamicRender"],
      ["preamble", "config", "template", "datasets", "postamble"],
    ]);
  });

  it("gives nothing for a form without XFA", () => {
    using doc = open("fields.pdf");
    expect(doc.xfa()).toBeUndefined();
  });

  it("removes the part with one edit, and undo gives the packets back byte for byte", () => {
    using doc = open("xfa-static.pdf");
    const before = doc.xfa()?.packets.map((p) => text(p.bytes));
    doc.apply([{ op: "removeXfa", args: {} }]);
    const removed = [doc.xfa(), doc.formFields().hasXfa, doc.formFields().fields.length];
    doc.undo();
    expect([removed, doc.xfa()?.packets.map((p) => text(p.bytes))]).toEqual([[undefined, false, 1], before]);
  });

  it("refuses to remove a part the form doesn't have", () => {
    using doc = open("fields.pdf");
    const error = thrown(() => doc.apply([{ op: "removeXfa", args: {} }]));
    expect(isLaminaPdfError(error) && error.code).toBe("invalid-op");
  });
});
