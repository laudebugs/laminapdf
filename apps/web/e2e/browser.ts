// The browser each e2e run uses (LAMINA_PDF_E2E_BROWSER: chromium, the default, firefox or webkit),
// shared by the suites so each launches it the same way.
import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, webkit } from "playwright";

const APP = new URL("../", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };

/** This run's browser's name. */
export const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";

/**
 * Launches this run's browser, able to run the reader's service worker from the e2e server, whose
 * certificate is self-signed: a page's `ignoreHTTPSErrors` doesn't cover a service worker's script,
 * so Chromium ignores certificate errors and Firefox runs service workers in testing mode. The
 * deployed reader needs neither; these switches only stand in for a trusted certificate.
 */
export function launch(): Promise<Browser> {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  if (BROWSER === "firefox") {
    return type.launch({ env: firefoxEnv(APP), firefoxUserPrefs: { "dom.serviceWorkers.testing.enabled": true } });
  }
  return type.launch(BROWSER === "chromium" ? { args: ["--ignore-certificate-errors"] } : {});
}
