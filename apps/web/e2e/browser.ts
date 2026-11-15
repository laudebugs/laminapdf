// The browser each e2e run uses (@laminapdf/test-utils/e2e-browser), with the switches the reader's
// service worker needs.
import { BROWSER, launchBrowser } from "@laminapdf/test-utils/e2e-browser";
import type { Browser } from "playwright";

export { BROWSER };

/**
 * Launches this run's browser, able to run the reader's service worker from the e2e server, whose
 * certificate is self-signed: a page's `ignoreHTTPSErrors` doesn't cover a service worker's script,
 * so Chromium ignores certificate errors and Firefox runs service workers in testing mode. The
 * deployed reader needs neither; these switches only stand in for a trusted certificate.
 */
export function launch(): Promise<Browser> {
  return launchBrowser(new URL("../", import.meta.url), {
    chromiumArgs: ["--ignore-certificate-errors"],
    firefoxUserPrefs: { "dom.serviceWorkers.testing.enabled": true },
  });
}
