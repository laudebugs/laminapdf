// knip: unused files, exports and dependencies across the packages and the docs site. Each
// package's entries are its `exports` (tools/ts/package-entries.mjs), plus what runs without being
// exported: the worker files started with `new URL(…)`, the test files, the files a test runner
// loads (Vite's app, workerd's Worker), Vitest configs, examples and benches. Test helpers are not
// entries, so an export only a helper's own file uses is reported. Run by `cargo xtask check ts`
// (`pnpm run check:unused`).
import { existsSync, readdirSync } from "node:fs";
import type { KnipConfig } from "knip";
import { entrySources, packageDirs } from "./tools/ts/package-entries.mjs";

/** Entry globs that apply where their folder or file exists. */
const RUNS_UNEXPORTED: Record<string, string[]> = {
  "src/worker/entry.ts": ["src/worker/entry.ts"],
  test: ["test/**/*.test.{ts,tsx}"],
  "test/vite-app": ["test/vite-app/*.ts"],
  "test/workerd/worker.ts": ["test/workerd/worker.ts"],
  examples: ["examples/**/*.{ts,tsx,mjs}"],
  bench: ["bench/**/*.mjs"],
  scripts: ["scripts/**/*.mjs"],
};

/** Whether `packages/<dir>/<sub>` holds a file ending in `ext`. */
function holds(dir: string, sub: string, ext: string): boolean {
  const path = `packages/${dir}/${sub}`;
  return existsSync(path) && readdirSync(path).some((f) => f.endsWith(ext));
}

function entries(dir: string): string[] {
  const at = (p: string) => (p === "scripts" ? holds(dir, p, ".mjs") : existsSync(`packages/${dir}/${p}`));
  const runs = Object.entries(RUNS_UNEXPORTED).flatMap(([p, globs]) => (at(p) ? globs : []));
  const vitest = readdirSync(`packages/${dir}`).some((f) => /^vitest\..*config\.ts$/.test(f))
    ? ["vitest.*config.ts"]
    : [];
  return [...entrySources(".", dir), ...runs, ...vitest];
}

function project(dir: string): string[] {
  // A TypeScript script (run by a package.json script, which knip reads as the entry) is project code.
  const tsScripts = holds(dir, "scripts", ".ts");
  return [
    ...["src/**/*.{ts,tsx}", "test/**/*.{ts,tsx}", "examples/**/*.{ts,tsx,mjs}"].filter((g) =>
      existsSync(`packages/${dir}/${g.split("/")[0]}`),
    ),
    ...(tsScripts ? ["scripts/**/*.ts"] : []),
  ];
}

const config: KnipConfig = {
  workspaces: {
    // The root has no code of its own: its tools (tools/, docs/ research) are run by xtask.
    ".": { entry: [], project: [] },
    ...Object.fromEntries(
      packageDirs(".").map((dir) => [`packages/${dir}`, { entry: entries(dir), project: project(dir) }]),
    ),
    // Its tests are named here: knip can't load its Vitest config, which loads Astro's whole config.
    "apps/site": { entry: ["test/**/*.test.ts", "vitest.config.ts"], vitest: false },
    // The docs' example runner: scripts/build-runner.mjs builds runner/worker.ts by its path.
    // src/routeData.ts is Starlight's route middleware, which astro.config.mjs names by path.
    "apps/docs": { entry: ["runner/worker.ts", "src/routeData.ts"] },
    // The desktop reader's e2e scenarios. WebdriverIO's config sits in e2e/, not the package root
    // where knip's plugin looks, and it builds the specs' paths, which knip can't follow.
    "apps/desktop": {
      entry: ["e2e/specs/*.e2e.ts"],
      "webdriver-io": { config: ["e2e/wdio.conf.ts", "e2e/wdio.cold-open.conf.ts"] },
      // The config's `runner: "local"` loads it; knip's plugin doesn't map the name to the package.
      ignoreDependencies: ["@wdio/local-runner"],
    },
    // A command, not a library: no `exports`. bin/laminapdf.js runs the built src/launcher.ts.
    "packages/cli": { entry: ["src/launcher.ts"] },
    // Its browser tests' config loads Astro, which knip can't, so its test entries are named here.
    "packages/site-kit": {
      entry: ["test/**/*.test.ts", "test/browser/render.ts", "vitest.config.ts", "vitest.browser.config.ts"],
      vitest: false,
    },
  },
  // Build output is never project code: apps/web imports the built viewer, whose dist/ exists after
  // `cargo xtask wasm`, and knip would follow the import into it.
  ignore: ["**/src/generated/**", "**/dist/**"],
  // knip finds pnpm's workspaces itself; the generated package is left out here too.
  ignoreWorkspaces: ["packages/wasm"],
  ignoreDependencies: [
    // Run by `cargo xtask pdf-runtimes` as `pnpm exec deno` and `pnpm exec bun`, and by `cargo xtask
    // semver` as `pnpm --filter @laminapdf/sdk exec api-extractor`, which knip can't see.
    "deno",
    "bun",
    "@microsoft/api-extractor",
  ],
  // apps/web's e2e remote host makes its certificate with the system's openssl (e2e/remote-host.ts).
  ignoreBinaries: ["openssl"],
};

export default config;
