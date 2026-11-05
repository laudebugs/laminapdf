// `pnpm --filter @laminapdf/icons lint:icons [file.svg…]`: lints every icon in svg/ (or the files
// named), printing each error and each icon whose margin is under the design language's target.

import { readdirSync, readFileSync } from "node:fs";
import { basename } from "node:path";
import process from "node:process";
import { lintIcon } from "./rules.ts";

const dir = new URL("../svg/", import.meta.url);
const named = process.argv.slice(2);
const files =
  named.length > 0
    ? named
    : readdirSync(dir)
        .filter((f) => f.endsWith(".svg"))
        .map((f) => new URL(f, dir).pathname);
let errors = 0;
for (const file of files) {
  const { errors: e, warnings } = lintIcon(basename(file, ".svg"), readFileSync(file, "utf8"));
  for (const line of e) console.error(`error: ${line}`);
  for (const line of warnings) console.warn(`warning: ${line}`);
  errors += e.length;
}
console.log(`${files.length} icons, ${errors} error(s)`);
process.exitCode = errors > 0 ? 1 : 0;
