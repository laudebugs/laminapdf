// runExample (run.ts) with a fake SDK: an example's export is called with its inputs and its bytes
// come back, returned or handed to `save` as a quick start does; each way a reader's edit can go wrong comes back as a code and a message (and a
// LaminaPdfError's hint), never as a thrown error. Run with `node --test`.
import assert from "node:assert/strict";
import { test } from "node:test";
import { runExample, SAVE } from "./run.ts";

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

test("an example's export runs with its inputs, types stripped", async () => {
  const outcome = await runExample(EXAMPLE, "hello", ["docs", { suffix: "!" }], sdk);
  assert.equal(outcome.ok, true);
  assert.equal(outcome.ok && new TextDecoder().decode(outcome.bytes), "Hello, docs!");
});

test("a LaminaPdfError comes back with its code and hint", async () => {
  const source = `import { fail } from "@laminapdf/sdk";\nexport function hello(): Uint8Array { return fail(); }\n`;
  assert.deepEqual(await runExample(source, "hello", [], sdk), {
    ok: false,
    code: "invalid-geometry",
    message: "invalid page geometry",
    hint: "use positive sizes",
  });
});

test("each other mistake has its own code", async () => {
  /** @type {[string, string][]} */
  const cases = [
    ["export function hello( {", "syntax"],
    ["export function goodbye(): Uint8Array { return new Uint8Array(); }", "missing-export"],
    ['export function hello(): string { return "not a PDF"; }', "not-a-pdf"],
    ['import { readFileSync } from "node:fs";\nexport function hello() { return readFileSync("x"); }', "error"],
    ["export function hello(): Uint8Array { return (undefined as any).x; }", "error"],
  ];
  for (const [source, code] of cases) {
    const outcome = await runExample(source, "hello", [], sdk);
    assert.equal(outcome.ok, false, source);
    assert.equal(!outcome.ok && outcome.code, code, source);
  }
});

test("only @laminapdf/sdk and node:fs/promises may be imported", async () => {
  const outcome = await runExample('import "node:fs";\nexport function hello() {}', "hello", [], sdk);
  assert.equal(
    !outcome.ok && outcome.message,
    "the example may import only @laminapdf/sdk and writeFile from node:fs/promises, not node:fs",
  );
});

/** A quick start as packages/sdk/examples/quick-start/node.ts writes it, over the fake SDK. */
const QUICK_START = `import { writeFile } from "node:fs/promises";
import { make } from "@laminapdf/sdk";
export async function hello(
  save: (pdf: Uint8Array) => void | Promise<void> = (pdf) => writeFile("hello.pdf", pdf),
  options: { suffix: string } = { suffix: "" },
): Promise<void> {
  await save(make(\`Hello\${options.suffix}\`));
}
if (import.meta.main) await hello();
`;

test("a quick start's PDF is what it hands to save", async () => {
  const outcome = await runExample(QUICK_START, "hello", [SAVE, { suffix: "!" }], sdk);
  assert.equal(outcome.ok && new TextDecoder().decode(outcome.bytes), "Hello!");
});

test("a quick start that writes its file instead shows what it wrote", async () => {
  const outcome = await runExample(QUICK_START, "hello", [], sdk);
  assert.equal(outcome.ok && new TextDecoder().decode(outcome.bytes), "Hello");
});

test("another member of node:fs/promises is refused by name", async () => {
  const source =
    'import { readFile } from "node:fs/promises";\nexport async function hello() { return readFile("x"); }';
  const outcome = await runExample(source, "hello", [], sdk);
  assert.equal(
    !outcome.ok && outcome.message,
    "the example may import only @laminapdf/sdk and writeFile from node:fs/promises, not readFile from node:fs/promises",
  );
});

test("a saved PDF counts whatever the function returns", async () => {
  const source = `import { make } from "@laminapdf/sdk";
export function hello(save: (pdf: Uint8Array) => void): boolean {
  save(make("Saved"));
  return true;
}
`;
  const outcome = await runExample(source, "hello", [SAVE], sdk);
  assert.equal(outcome.ok && new TextDecoder().decode(outcome.bytes), "Saved");
});

test("a quick start that saves nothing is not a PDF", async () => {
  const source = QUICK_START.replace("await save(", "void (");
  const outcome = await runExample(source, "hello", [SAVE], sdk);
  assert.equal(!outcome.ok && outcome.code, "not-a-pdf");
});
