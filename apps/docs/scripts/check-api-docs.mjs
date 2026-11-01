// Fails when a package entry exports something without a doc comment: TypeDoc's `notDocumented`
// validation, with its warnings treated as errors, over every entry each package's `exports`
// points at (every subpath and condition), so a new entry is checked without editing this file.
// The packages and entries come from tools/ts/package-entries.mjs, which knip reads too.
// Run by `cargo xtask check ts` (`pnpm --filter @laminapdf/docs run check:api-docs`).

import process from "node:process";
import { fileURLToPath } from "node:url";
import { Application, LogLevel } from "typedoc";
import { entrySources, manifest, packageDirs } from "../../../tools/ts/package-entries.mjs";

const ROOT = fileURLToPath(new URL("../../../", import.meta.url));

/**
 * A package's npm name, and its build tsconfig and entry sources as absolute paths.
 *
 * @param {string} dir
 */
function entriesOf(dir) {
  return {
    name: manifest(ROOT, dir).name,
    tsconfig: `${ROOT}packages/${dir}/tsconfig.build.json`,
    entries: entrySources(ROOT, dir).map((f) => `${ROOT}packages/${dir}/${f}`),
  };
}

let failed = 0;
for (const pkg of packageDirs(ROOT).map(entriesOf)) {
  const app = await Application.bootstrap({
    entryPoints: pkg.entries,
    tsconfig: pkg.tsconfig,
    // Type errors are tsc's job (and the wasm-bindgen types need `cargo xtask wasm` first).
    readme: "none",
    skipErrorChecking: true,
    excludeInternal: true,
    excludePrivate: true,
    excludeProtected: true,
    excludeExternals: true,
    validation: { notExported: false, invalidLink: false, notDocumented: true },
    // The guideline's "exported symbols and public methods": declarations, not every property.
    requiredToBeDocumented: ["Enum", "Variable", "Function", "Class", "Interface", "TypeAlias", "Method", "Accessor"],
    treatValidationWarningsAsErrors: true,
    logLevel: LogLevel.Warn,
  });
  const project = await app.convert();
  if (project === undefined) {
    console.error(`${pkg.name}: TypeDoc could not convert the entries`);
    failed++;
    continue;
  }
  app.validate(project);
  const warnings = app.logger.warningCount + app.logger.errorCount;
  if (warnings > 0) {
    console.error(`${pkg.name}: ${warnings} undocumented export(s), listed above`);
    failed++;
  } else console.log(`${pkg.name}: every export is documented`);
}
process.exitCode = failed > 0 ? 1 : 0;
