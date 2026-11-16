// The Properties dialog's Apply as ops (O04): the XMP tab's changes, then the Description form's,
// with `replaceMalformed` on every op that can take it when the XMP metadata can't be read.

import { describe, expect, it } from "vitest";
import { propertiesOps } from "../src/reader/properties-ops.ts";

const DC = "http://purl.org/dc/elements/1.1/";

describe("propertiesOps", () => {
  it("makes the form's change one setMetadata and one removeMetadata", () => {
    const ops = propertiesOps({ patch: { title: "Report" }, remove: ["subject"] }, [], { replaceMalformed: false });
    expect(ops).toEqual([
      { op: "setMetadata", args: { patch: { title: "Report" } } },
      { op: "removeMetadata", args: { keys: ["subject"] } },
    ]);
  });

  it("puts the XMP tab's changes first, then the form's", () => {
    const ops = propertiesOps(
      { patch: { title: "Report" }, remove: [] },
      [{ kind: "remove", namespace: DC, name: "rights" }],
      {
        replaceMalformed: false,
      },
    );
    expect(ops.map((op) => op.op)).toEqual(["removeXmpProperty", "setMetadata"]);
  });

  it("asks every property op to replace unreadable XMP metadata, but not a replaced packet", () => {
    const ops = propertiesOps(
      { patch: { title: "Report" }, remove: [] },
      [
        { kind: "set", namespace: DC, prefix: "dc", name: "rights", value: { type: "text", value: "CC0" } },
        { kind: "replace", xml: "<x:xmpmeta/>" },
      ],
      { replaceMalformed: true },
    );
    expect(ops.map((op) => "replaceMalformed" in op.args)).toEqual([true, false, true]);
  });

  it("gives no op for a form with nothing changed", () => {
    expect(propertiesOps(undefined, [], { replaceMalformed: false })).toEqual([]);
  });
});
