// runExample (run.ts) with a fake SDK: an example's export is called with its inputs and its bytes
// come back; each way a reader's edit can go wrong comes back as a code and a message (and a
// LaminaPdfError's hint), never as a thrown error. Run with `node --test`.
import assert from "node:assert/strict";
import { test } from "node:test";
import { runExample } from "./run.ts";

/** A stand-in for @laminapdf/sdk: `make(text)` returns the text's bytes; `fail()` throws as the SDK does. */
const sdk = {
  /** @param {string} text */
  make: (text) => new TextEncoder().encode(text),
  fail: () => {
    throw Object.assign(new Error("invalid page geometry"), { code: "invalid-geometry", hint: "use positive sizes" });
  },
};

const EXAMPLE = `import { make } from "@laminapdf/sdk";
export function hello(name: string, options: { suffix: string } = { suffix: "" }): Uint8Array {
  return make(\`Hello, \${name}\${options.suffix}\`);
}
`;

test("an example's export runs with its inputs, types stripped", () => {
  const outcome = runExample(EXAMPLE, "hello", ["docs", { suffix: "!" }], sdk);
  assert.equal(outcome.ok, true);
  assert.equal(outcome.ok && new TextDecoder().decode(outcome.bytes), "Hello, docs!");
});

test("a LaminaPdfError comes back with its code and hint", () => {
  const source = `import { fail } from "@laminapdf/sdk";\nexport function hello(): Uint8Array { return fail(); }\n`;
  assert.deepEqual(runExample(source, "hello", [], sdk), {
    ok: false,
    code: "invalid-geometry",
    message: "invalid page geometry",
    hint: "use positive sizes",
  });
});

test("each other mistake has its own code", () => {
  /** @type {[string, string][]} */
  const cases = [
    ["export function hello( {", "syntax"],
    ["export function goodbye(): Uint8Array { return new Uint8Array(); }", "missing-export"],
    ['export function hello(): string { return "not a PDF"; }', "not-a-pdf"],
    ['import { readFileSync } from "node:fs";\nexport function hello() { return readFileSync("x"); }', "error"],
    ["export function hello(): Uint8Array { return (undefined as any).x; }", "error"],
  ];
  for (const [source, code] of cases) {
    const outcome = runExample(source, "hello", [], sdk);
    assert.equal(outcome.ok, false, source);
    assert.equal(!outcome.ok && outcome.code, code, source);
  }
});

test("only @laminapdf/sdk may be imported", () => {
  const outcome = runExample('import "node:fs";\nexport function hello() {}', "hello", [], sdk);
  assert.equal(!outcome.ok && outcome.message, "the example may import only @laminapdf/sdk, not node:fs");
});
