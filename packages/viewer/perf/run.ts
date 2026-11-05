// `pnpm --filter @laminapdf/viewer perf [-- --browsers chromium,firefox,webkit] [--quick] [--out <dir>]`:
// the viewer's performance harness. It builds the fixtures (fixtures.ts) and the harness page
// (page/, with Vite, over the built package: run `cargo xtask wasm` first), serves it (serve.ts), runs
// the scenarios (scenarios.ts) in each browser and writes `<out>/<browser>.json` and
// `<out>/summary.md` (default out: the repository's target/viewer-perf/). It reports the targets
// it misses and never fails on one (ADR 0018: no wall-clock assertions).

import { mkdirSync, writeFileSync } from "node:fs";
import { cpus, release, totalmem, type } from "node:os";
import { argv } from "node:process";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type BrowserType, chromium, firefox, webkit } from "playwright";
import { build } from "vite";
import { ensureFixtures } from "./fixtures.ts";
import { type BrowserResult, historyEntries, type Machine, summary } from "./report.ts";
import { runScenarios } from "./scenarios.ts";
import { serve } from "./serve.ts";

const here = (path: string) => new URL(path, import.meta.url);
const BROWSERS: Readonly<Record<string, BrowserType>> = { chromium, firefox, webkit };

const { values } = parseArgs({
  // pnpm passes a `--` before the script's arguments through.
  args: argv.slice(2).filter((arg, i) => !(i === 0 && arg === "--")),
  options: {
    browsers: { type: "string", default: "chromium,firefox,webkit" },
    quick: { type: "boolean", default: false },
    out: { type: "string", default: fileURLToPath(here("../../../target/viewer-perf/")) },
  },
});
const names = values.browsers.split(",").filter((name) => name !== "");
for (const name of names) if (BROWSERS[name] === undefined) throw new Error(`unknown browser ${name}`);

const fixtures = await ensureFixtures(here(".fixtures/"));
const dist = fileURLToPath(here(".dist/"));
await build({
  root: fileURLToPath(here("page/")),
  publicDir: fileURLToPath(fixtures),
  logLevel: "warn",
  build: { outDir: dist, emptyOutDir: true, target: "es2022" },
});
const server = await serve(dist);
const { url } = server;

const machine: Machine = {
  cpu: cpus()[0]?.model.trim() ?? "unknown CPU",
  cores: cpus().length,
  memoryBytes: totalmem(),
  os: `${type()} ${release()}`,
};
mkdirSync(values.out, { recursive: true });
const results: BrowserResult[] = [];
try {
  for (const name of names) {
    const launcher = BROWSERS[name];
    if (launcher === undefined) continue;
    // Chromium rounds `performance.memory` unless asked for precise figures.
    const options =
      name === "firefox"
        ? { env: firefoxEnv(here("../")) }
        : name === "chromium"
          ? { args: ["--enable-precise-memory-info"] }
          : {};
    const browser = await launcher.launch(options);
    try {
      console.log(`viewer perf: ${name} ${browser.version()}`);
      const measured = await runScenarios(browser, name, url, values.quick);
      const result: BrowserResult = {
        browser: name,
        version: browser.version(),
        quick: values.quick,
        machine,
        ...measured,
      };
      results.push(result);
      const json = { ...result, history: historyEntries(result) };
      writeFileSync(`${values.out}/${name}.json`, `${JSON.stringify(json, null, 2)}\n`);
    } finally {
      await browser.close();
    }
  }
} finally {
  await server.close();
}
const report = summary(results);
writeFileSync(`${values.out}/summary.md`, report);
console.log(report);
