// WebdriverIO against the desktop reader's e2e build: `pnpm --filter @laminapdf/desktop
// desktop:build:e2e`, then `pnpm --filter @laminapdf/desktop e2e`. The WebDriver server runs
// inside the app (tauri-plugin-wdio-webdriver), on every OS, so no driver is installed.
import { appendFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { describeFlaky, warningLine } from "@laminapdf/test-utils/flaky";
import { appBinary, output } from "./paths.ts";

const specs = join(fileURLToPath(new URL(".", import.meta.url)), "specs");
mkdirSync(output, { recursive: true });

// @wdio/tauri-service reads `tauri:options` but doesn't declare it in WebdriverIO's types.
const app = { browserName: "tauri", "tauri:options": { application: appBinary } };

export const config: WebdriverIO.Config = {
  runner: "local",
  // The service starts the app once for the whole run, so the scenarios share it and run in this
  // order: the empty state while nothing is open, then each scenario opens its own file, which
  // replaces the one before, so none depends on another's state.
  specs: [
    ["start", "open", "scroll", "zoom", "password", "repaired", "restart", "large"].map((name) =>
      join(specs, `${name}.e2e.ts`),
    ),
  ],
  maxInstances: 1,
  capabilities: [app],
  services: [["@wdio/tauri-service", { driverProvider: "embedded", captureBackendLogs: true, logDir: output }]],
  framework: "mocha",
  reporters: ["spec"],
  logLevel: "warn",
  outputDir: output,
  waitforTimeout: 10_000,
  // One retry, and a pass on it reported as flaky, never hidden (ADR 0018 §4).
  mochaOpts: { ui: "bdd", timeout: 60_000, retries: 1 },
  afterTest: async (test, _context, { passed, retries }) => {
    if (!passed) await browser.saveScreenshot(join(output, `${test.title.replace(/\W+/g, "-")}.png`));
    if (passed && retries.attempts > 0) reportFlaky(test.file, test.fullTitle);
  },
};

/** A test that passed only on its retry: a warning on Actions and `flaky=true` in the step's outputs. */
function reportFlaky(file: string, name: string): void {
  const flaky = { project: "desktop-e2e", file, name };
  if (env.GITHUB_ACTIONS !== "true") {
    console.warn(`flaky: ${describeFlaky(flaky)} failed, then passed on its retry`);
    return;
  }
  console.log(warningLine(flaky));
  if (env.GITHUB_OUTPUT !== undefined) appendFileSync(env.GITHUB_OUTPUT, "flaky=true\n");
}
