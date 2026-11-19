import type { FormField, Violation } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { accessFrom, isFilled, requiredCount, tabStops } from "./rules.ts";

/** A field of `kind` with widgets on `pages`, named `id`. */
function field(id: string, kind: FormField["kind"], extra: Partial<FormField> = {}, pages: number[] = [0]): FormField {
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
    widgets: pages.map((page, i) => ({ id: `${id}-w${i}`, page, rect: [0, 0, 10, 10], flags: 0 })),
    ...extra,
  } as FormField;
}

/** A violation of `restriction`, by the signature in field `signature`. */
function violation(restriction: Violation["restriction"], extra: Partial<Violation> = {}): Violation {
  return {
    class: "formFill",
    restriction,
    signature: { kind: "approval", field: "sig" },
    ...extra,
  } as Violation;
}

describe("tabStops", () => {
  const fields = [
    field("name", "text"),
    field("colour", "radioGroup", {}, [0, 0]),
    field("go", "pushButton"),
    field("notes", "text", {}, [1]),
  ];

  it("follows each page's order, pages in turn", () => {
    const pages = new Map([
      [1, [{ widget: "notes-w0", field: "notes" }]],
      [0, [{ widget: "name-w0", field: "name" }]],
    ]);
    expect(tabStops(fields, pages).map((s) => s.field)).toEqual(["name", "notes"]);
  });

  it("visits a radio group once and leaves out push buttons", () => {
    const pages = new Map([
      [
        0,
        [
          { widget: "colour-w0", field: "colour" },
          { widget: "go-w0", field: "go" },
          { widget: "colour-w1", field: "colour" },
        ],
      ],
    ]);
    expect(tabStops(fields, pages).map((s) => s.widget)).toEqual(["colour-w0"]);
  });
});

describe("accessFrom", () => {
  const fields = [field("name", "text"), field("address", "text"), field("sig", "signature")];

  it("locks the fields a signature names, naming the signature's field", () => {
    const { access, locks, banner } = accessFrom(fields, [violation("fieldMdp", { field: "name" })]);
    expect([access.get("name"), access.get("address"), locks.get("name"), banner]).toEqual([
      { kind: "locked", reason: "Locked by the signature in “sig”" },
      undefined,
      "Locked by the signature in “sig”",
      undefined,
    ]);
  });

  it("guards every field and shows the banner on a file certified with no changes allowed", () => {
    const violations = [violation("docMdp", { permission: 1 })];
    const { access, banner } = accessFrom(fields, violations);
    expect([access.get("name"), access.get("address"), banner?.kind]).toEqual([
      { kind: "guarded" },
      { kind: "guarded" },
      "certified",
    ]);
  });

  it("leaves a file certified for form filling open", () => {
    const { access, banner } = accessFrom(fields, []);
    expect([access.size, banner]).toEqual([0, undefined]);
  });
});

describe("isFilled", () => {
  it("counts a value only when it holds something", () => {
    const fields = [
      field("a", "text", { value: { type: "text", value: "x" } }),
      field("b", "text", { value: { type: "text", value: "" } }),
      field("c", "checkBox", { value: { type: "button", value: null } }),
      field("d", "listBox", { value: { type: "choice", value: ["One"] } }),
    ];
    expect(fields.map(isFilled)).toEqual([true, false, false, true]);
  });
});

describe("requiredCount", () => {
  it("counts the required fields a person fills, and those that hold a value", () => {
    const fields = [
      field("a", "text", { required: true, value: { type: "text", value: "x" } }),
      field("b", "text", { required: true }),
      field("e", "pushButton", { required: true }),
    ];
    expect(requiredCount(fields)).toEqual({ filled: 1, total: 2 });
  });
});

describe("tabStops on hidden widgets", () => {
  it("leaves out a widget the page doesn't show", () => {
    const hidden = field("secret", "text");
    const shown = { ...hidden, widgets: [{ ...hidden.widgets[0], flags: 2 }] } as FormField;
    const pages = new Map([[0, [{ widget: "secret-w0", field: "secret" }]]]);
    expect(tabStops([shown], pages)).toEqual([]);
  });
});
