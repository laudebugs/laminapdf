// The browser each e2e run uses (LAMINA_PDF_E2E_BROWSER: chromium, the default, firefox or webkit),
// shared by the suites so each launches it the same way.
import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, webkit } from "playwright";

const APP = new URL("../", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };

/** This run's browser's name. */
export const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";

/** Launches this run's browser. */
export function launch(): Promise<Browser> {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  return type.launch(BROWSER === "firefox" ? { env: firefoxEnv(APP) } : {});
}
