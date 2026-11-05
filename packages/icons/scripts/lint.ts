// `pnpm --filter @laminapdf/icons lint:icons`: checks every icon in svg/ and aliases.json as the
// generator does, printing each error, and each icon whose margin is under the design language's
// target.

import process from "node:process";
import { aliases, iconFiles } from "./files.ts";
import { checkIcons } from "./icons.ts";
import { lintIcon } from "./rules.ts";

const files = iconFiles();
const errors = checkIcons(files, aliases());
for (const line of errors) console.error(`error: ${line}`);
for (const [name, text] of files) for (const line of lintIcon(name, text).warnings) console.warn(`warning: ${line}`);
console.log(`${files.length} icons, ${errors.length} error(s)`);
process.exitCode = errors.length > 0 ? 1 : 0;
