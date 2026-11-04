// Writes src/generated/tokens.css, src/generated/tokens.ts and CONTRAST.md from tokens.json.
// `--check` writes nothing and fails naming each file that differs, so a stale output fails the
// package's tests. Run with `pnpm --filter @laminapdf/tokens generate [--check]`.

import { readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { emitContrast, emitCss, emitTs } from "./emit.ts";
import { flatten, resolve } from "./model.ts";

const at = (path: string) => new URL(`../${path}`, import.meta.url);

const tokens = flatten(JSON.parse(readFileSync(at("tokens.json"), "utf8")));
for (const path of tokens.keys()) resolve(tokens, path);

const outputs: [string, string][] = [
  ["src/generated/tokens.css", emitCss(tokens)],
  ["src/generated/tokens.ts", emitTs(tokens)],
  ["CONTRAST.md", emitContrast(tokens)],
];

if (process.argv.includes("--check")) {
  const stale = outputs.filter(([path, text]) => {
    try {
      return readFileSync(at(path), "utf8") !== text;
    } catch {
      return true;
    }
  });
  for (const [path] of stale)
    console.error(`packages/tokens/${path} is stale: run \`pnpm --filter @laminapdf/tokens generate\``);
  process.exitCode = stale.length > 0 ? 1 : 0;
} else {
  for (const [path, text] of outputs) writeFileSync(at(path), text);
}
