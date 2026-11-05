// Writes src/generated/icons.ts from svg/*.svg and aliases.json, after linting every icon (an
// error stops it). `--check` writes nothing and fails when the committed module differs, so the
// package's tests catch a stale one. Run with `pnpm --filter @laminapdf/icons generate [--check]`.

import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { emitIcons, readIcons } from "./icons.ts";

const at = (path: string) => new URL(`../${path}`, import.meta.url);
const files = readdirSync(at("svg")).filter((f) => f.endsWith(".svg"));
const icons = readIcons(
  files.map((f) => [f.slice(0, -4), readFileSync(at(`svg/${f}`), "utf8")]),
  JSON.parse(readFileSync(at("aliases.json"), "utf8")),
);
const text = emitIcons(icons);
const out = "src/generated/icons.ts";

if (process.argv.includes("--check")) {
  let current = "";
  try {
    current = readFileSync(at(out), "utf8");
  } catch {}
  if (current !== text) {
    console.error(`packages/icons/${out} is stale: run \`pnpm --filter @laminapdf/icons generate\``);
    process.exitCode = 1;
  }
} else {
  writeFileSync(at(out), text);
}
