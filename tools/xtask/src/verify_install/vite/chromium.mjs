// `cargo xtask verify-install chromium`: builds the Vite app in this folder for production, serves
// it with `vite preview` and opens it in Playwright's Chromium, then prints the page's check line.
// Vite and Playwright are the repository's (pinned dev dependencies of @lamina/pdf), resolved
// from LAMINA_TOOLS_FROM; the app's own dependencies are only the installed tarballs.

import { createRequire } from "node:module";
import { cwd, env } from "node:process";
import { pathToFileURL } from "node:url";
import { freePort } from "./free-port.mjs";

const from = createRequire(env.LAMINA_TOOLS_FROM ?? "");
const { build, preview } = await import(pathToFileURL(from.resolve("vite")).href);
// playwright's entry is CommonJS: its exports arrive as the module's default.
const playwright = await import(pathToFileURL(from.resolve("playwright")).href);
const { chromium } = playwright.default ?? playwright;

const root = cwd();
await build({ root, configFile: false, logLevel: "warn" });
const port = await freePort();
const server = await preview({
  root,
  configFile: false,
  logLevel: "warn",
  preview: { host: "127.0.0.1", port, strictPort: true },
});
const browser = await chromium.launch();
try {
  console.log(`LAMINA_VERIFY_VERSION chromium ${browser.version()}`);
  const page = await browser.newPage();
  page.on("console", (message) => console.error(`[chromium] ${message.text()}`));
  page.on("pageerror", (error) => console.error(`[chromium] ${error}`));
  await page.goto(`http://127.0.0.1:${port}/`);
  const line = await page.waitForFunction(() => globalThis.laminaVerify, null, { timeout: 180_000 });
  console.log(await line.jsonValue());
} finally {
  await browser.close();
  await server.close();
}
