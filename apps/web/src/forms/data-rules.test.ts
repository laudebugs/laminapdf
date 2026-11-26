import type { FormField } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { exportCounts, keepFilledJson, planFlatten, planReset } from "./data-rules.ts";

/** A field of `kind` named `id`, with an optional value and default. */
function field(id: string, kind: FormField["kind"], extra: Partial<FormField> = {}): FormField {
  return {
    id,
    shared: [],
    name: id,
    kind,
    flags: 0,
    readOnly: false,
    required: false,
    options: [],
    onStates: [],
    signed: false,
    widgets: [],
    ...extra,
  } as FormField;
}

const text = (value: string) => ({ type: "text", value }) as const;

describe("planReset", () => {
  it("resets every answer that isn't its default and leaves locked fields alone", () => {
    const fields = [
      field("f1", "text", { value: text("Ada") }),
      field("f2", "text", { value: text("x"), defaultValue: text("x") }),
      field("f3", "text", { value: text("Grace") }),
      field("f4", "signature"),
      field("f5", "pushButton"),
    ];
    const plan = planReset(fields, new Map([["f3", "locked"]]));
    expect(plan.fields).toBe(3);
    expect(plan.locked).toBe(1);
    expect(plan.ops).toEqual([{ op: "setFieldValue", args: { field: "f1", value: { type: "clear" } } }]);
  });

  it("gives a field back its default value", () => {
    const plan = planReset([field("f1", "text", { value: text("Ada"), defaultValue: text("n/a") })], new Map());
    expect(plan.ops).toEqual([{ op: "setFieldValue", args: { field: "f1", value: text("n/a") } }]);
  });
});

describe("planFlatten", () => {
  it("flattens every field but signatures and locked ones", () => {
    const fields = [field("f1", "text"), field("f2", "checkBox"), field("f3", "signature"), field("f4", "text")];
    const plan = planFlatten(fields, new Map([["f4", "locked"]]));
    expect(plan.ids).toEqual(["f1", "f2"]);
    expect(plan.locked).toBe(1);
    expect(plan.hasSignatureField).toBe(true);
    expect(plan.op).toEqual({ op: "flattenForm", args: { fields: ["f1", "f2"], intent: "view" } });
  });

  it("has no op when nothing is left to flatten", () => {
    expect(planFlatten([field("f1", "signature")], new Map()).op).toBeUndefined();
  });
});

describe("export", () => {
  it("counts the fields with an answer out of the fields it lists", () => {
    const fields = [field("f1", "text", { value: text("Ada") }), field("f2", "text"), field("f3", "signature")];
    expect(exportCounts(fields)).toEqual({ filled: 1, total: 2 });
  });

  it("keeps only the answered fields of a JSON export", () => {
    const json = JSON.stringify({ $schema: "s", fields: { a: "Ada", b: "", c: null, d: false, e: true, f: [] } });
    const kept = JSON.parse(new TextDecoder().decode(keepFilledJson(new TextEncoder().encode(json))));
    expect(kept).toEqual({ $schema: "s", fields: { a: "Ada", e: true } });
  });
});
