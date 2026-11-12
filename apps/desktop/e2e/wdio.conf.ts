// WebdriverIO against the desktop reader's e2e build (D2e): `pnpm --filter @laminapdf/desktop
// desktop:build:e2e`, then `pnpm --filter @laminapdf/desktop e2e`. The WebDriver server runs
// inside the app (tauri-plugin-wdio-webdriver), on every OS, so no driver is installed.
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const root = join(here, "..", "..", "..");
const binary = join(root, "target", "debug", process.platform === "win32" ? "laminapdf-desktop.exe" : "laminapdf-desktop");
/** Screenshots and logs of failed scenarios, which CI uploads. */
const output = join(root, "target", "desktop-e2e");
mkdirSync(output, { recursive: true });

export const config: WebdriverIO.Config = {
  runner: "local",
  // The service starts the app once for the whole run, so the scenarios share it and run in this
  // order: the empty state while nothing is open, then each scenario opens its own file, which
  // replaces the one before.
  specs: [["start", "open", "scroll", "zoom", "password", "repaired", "restart", "large"].map((name) => join(here, "specs", `${name}.e2e.ts`))],
  maxInstances: 1,
  capabilities: [{ browserName: "tauri", "tauri:options": { application: binary } }],
  services: [["@wdio/tauri-service", { driverProvider: "embedded", captureBackendLogs: true, logDir: output }]],
  framework: "mocha",
  reporters: ["spec"],
  logLevel: "warn",
  outputDir: output,
  waitforTimeout: 10_000,
  mochaOpts: { ui: "bdd", timeout: 60_000 },
  afterTest: async (test, _context, { passed }) => {
    if (!passed) await browser.saveScreenshot(join(output, `${test.title.replace(/\W+/g, "-")}.png`));
  },
};
