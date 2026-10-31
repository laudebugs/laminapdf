// ADR 0003 §4 and ADR 0026, TS half: the generated tables are fixtures/codes.json's, every code
// has its explain page, LaminaError.toJSON() validates against lamina.error/1, and the shared
// cases of fixtures/errors/contract.json (ADR 0018 §5) throw exactly the object the Rust façade
// serialises (crates/lamina/tests/error_contract.rs checks the same file).
import { Document, init, LaminaError, type OpenOptions } from "@laminapdf/sdk";
import { DOCS_URL, docsUrl, ERROR_CODES, explain, HINTS, isCode, KINDS, WARNING_CODES } from "@laminapdf/sdk/explain";
import { Ajv2020 } from "ajv/dist/2020.js";
import { beforeAll, describe, expect, it } from "vitest";
import { ERROR_GROUPS, RAISED_BY_PACKAGE, WARNING_GROUPS } from "../src/generated/codes.js";
import { read, readText, thrown } from "./helpers.js";

interface Entry {
  readonly code: string;
  readonly kind: string;
  readonly summary: string;
  readonly hint: string;
}
const codes = JSON.parse(readText("fixtures/codes.json")) as Record<string, unknown>;
// A group of codes is a list of objects with a `code`; which ones list warnings comes from the
// generator (code_groups.rs), so the rule is written once.
const isGroup = (value: unknown): value is Entry[] =>
  Array.isArray(value) && value.every((e) => typeof e === "object" && e !== null && "code" in e);
const groups = Object.entries(codes).filter((entry): entry is [string, Entry[]] => isGroup(entry[1]));
const all = groups.flatMap(([, list]) => list);
const isWarningGroup = (group: string) => WARNING_GROUPS.includes(group);

const schema = JSON.parse(readText("packages/sdk/schemas/error.schema.json")) as object;
const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
const errorsOf = () => JSON.stringify(validate.errors);

beforeAll(() => init());

describe("generated codes", () => {
  it("knows every group of codes.json", () => {
    expect(groups.map(([g]) => g).sort()).toEqual([...ERROR_GROUPS, ...WARNING_GROUPS].sort());
  });

  it("lists every error code of codes.json", () => {
    const errors = groups.filter(([g]) => !isWarningGroup(g)).flatMap(([, l]) => l.map((e) => e.code));
    expect([...ERROR_CODES].sort()).toEqual(errors.sort());
  });

  it("lists every warning code of codes.json", () => {
    const warnings = groups.filter(([g]) => isWarningGroup(g)).flatMap(([, l]) => l.map((e) => e.code));
    expect([...WARNING_CODES].sort()).toEqual(warnings.sort());
  });

  it("gives every code codes.json's kind and hint", () => {
    const fromJson = Object.fromEntries(all.map((e) => [e.code, [e.kind, e.hint]]));
    const generated = Object.fromEntries(Object.keys(KINDS).map((c) => [c, [KINDS[c as never], HINTS[c as never]]]));
    expect(generated).toEqual(fromJson);
  });

  it("gives the package's own codes codes.json's kind and hint", () => {
    for (const [code, { kind, hint }] of Object.entries(RAISED_BY_PACKAGE)) {
      const entry = all.find((e) => e.code === code);
      expect([entry?.kind, entry?.hint]).toEqual([kind, hint]);
    }
  });

  it("builds docsUrl from codes.json's constant", () => {
    expect([DOCS_URL, docsUrl("wrong-password")]).toEqual([
      codes.docsUrl,
      `${(codes.docsUrl as { base: string }).base}wrong-password.md`,
    ]);
  });
});

describe("@laminapdf/sdk/explain", () => {
  it("returns docs/errors/<code>.md for every code", () => {
    const wrong = all.filter((e) => explain(e.code) !== readText(`docs/errors/${e.code}.md`)).map((e) => e.code);
    expect(wrong).toEqual([]);
  });

  it("returns undefined for a string that is not a code", () => {
    expect([explain("not-a-code"), explain("toString"), isCode("__proto__")]).toEqual([undefined, undefined, false]);
  });
});

describe("lamina.error/1", () => {
  interface Case {
    readonly name: string;
    readonly file?: string;
    readonly text?: string;
    readonly options: OpenOptions;
    readonly call?: "pageCount";
    readonly expected: unknown;
  }
  const contract = JSON.parse(readText("fixtures/errors/contract.json")) as { cases: Case[] };

  for (const c of contract.cases) {
    it(`throws the shared object for ${c.name}`, () => {
      const bytes = c.file === undefined ? new TextEncoder().encode(c.text) : read(c.file);
      const e = thrown(() => {
        const doc = Document.open(bytes, c.options);
        if (c.call === "pageCount") return doc.pageCount;
        return doc;
      });
      const json = e instanceof LaminaError ? JSON.parse(JSON.stringify(e)) : e;
      expect([json, validate(json), errorsOf()]).toEqual([c.expected, true, "null"]);
    });
  }

  it("accepts a warning's object", () => {
    const doc = Document.open(read("fixtures/corpus/hello-broken.pdf"));
    const warnings = doc.warnings;
    expect([warnings.length > 0, warnings.every((w) => validate(w)), errorsOf()]).toEqual([true, true, "null"]);
  });

  it("accepts a binding error with didYouMean", () => {
    const e = thrown(() => Document.open(new Uint8Array(), { pasword: "x" } as OpenOptions));
    const json = (e as LaminaError).toJSON();
    expect([json.code, validate(json), errorsOf()]).toEqual(["unknown-option", true, "null"]);
  });

  it("leaves an empty didYouMean out of the error and its object", () => {
    const e = new LaminaError({
      code: "unknown-option",
      kind: "usage",
      message: "an options record has a key lamina does not know",
      didYouMean: [],
      docsUrl: docsUrl("unknown-option"),
    });
    const json = e.toJSON();
    expect([e.didYouMean, "didYouMean" in json, validate(json), errorsOf()]).toEqual([undefined, false, true, "null"]);
  });

  it("rejects an object whose code is not listed", () => {
    expect(validate({ code: "no-such-code", kind: "input", message: "m", docsUrl: docsUrl("not-pdf") })).toBe(false);
  });
});
