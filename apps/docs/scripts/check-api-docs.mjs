// Fails when a package entry exports something without a doc comment: TypeDoc's `notDocumented`
// validation, with its warnings treated as errors, over every published TypeScript entry point.
// Run by `cargo xtask check ts` (`pnpm --filter @laminapdf/docs run check:api-docs`).
import process from "node:process";
import { fileURLToPath } from "node:url";
import { Application, LogLevel } from "typedoc";

const ROOT = new URL("../../../", import.meta.url);
const path = (p) => fileURLToPath(new URL(p, ROOT));

/** Each package's entries (its `exports`) and the tsconfig that builds them. */
const PACKAGES = [
  {
    name: "@laminapdf/sdk",
    tsconfig: "packages/sdk/tsconfig.build.json",
    entries: [
      "packages/sdk/src/index.ts",
      "packages/sdk/src/explain.ts",
      "packages/sdk/src/worker/index.ts",
      "packages/sdk/src/worker/server.ts",
    ],
  },
  {
    name: "@laminapdf/render",
    tsconfig: "packages/render/tsconfig.build.json",
    entries: ["packages/render/src/index.ts"],
  },
  {
    name: "@laminapdf/render-pdfium",
    tsconfig: "packages/render-pdfium/tsconfig.build.json",
    entries: ["packages/render-pdfium/src/index.ts"],
  },
  {
    name: "@laminapdf/render-hayro",
    tsconfig: "packages/render-hayro/tsconfig.build.json",
    entries: ["packages/render-hayro/src/index.ts"],
  },
  {
    name: "@laminapdf/viewer",
    tsconfig: "packages/viewer/tsconfig.build.json",
    entries: ["packages/viewer/src/index.ts"],
  },
];

let failed = 0;
for (const pkg of PACKAGES) {
  const app = await Application.bootstrap({
    entryPoints: pkg.entries.map(path),
    tsconfig: path(pkg.tsconfig),
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
