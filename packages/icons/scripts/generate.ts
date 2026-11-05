// Writes src/generated/icons.ts from svg/*.svg and aliases.json, after checking every icon (an
// error stops it). `--check` writes nothing and fails when the committed module differs, so the
// package's tests catch a stale one. Run with `pnpm --filter @laminapdf/icons generate [--check]`.

import { readFileSync, writeFileSync } from "node:fs";
import process from "node:process";
import { aliases, iconFiles } from "./files.ts";
import { emitIcons, readIcons } from "./icons.ts";

const text = emitIcons(readIcons(iconFiles(), aliases()));
const out = new URL("../src/generated/icons.ts", import.meta.url);

if (process.argv.includes("--check")) {
  let current = "";
  try {
    current = readFileSync(out, "utf8");
  } catch {}
  if (current !== text) {
    console.error("packages/icons/src/generated/icons.ts is stale: run `pnpm --filter @laminapdf/icons generate`");
    process.exitCode = 1;
  }
} else {
  writeFileSync(out, text);
}
