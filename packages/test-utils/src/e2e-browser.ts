// The browser an e2e run uses (LAMINA_PDF_E2E_BROWSER: chromium, the default, firefox or webkit),
// launched the same way by the reader's, the docs' and the website's suites.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, type LaunchOptions, webkit } from "playwright";

const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };

/** This run's browser's name. */
export const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";

/** What a suite adds to the launch, for one engine or the other. */
export interface LaunchExtras {
  /** Chromium's command-line switches. */
  readonly chromiumArgs?: readonly string[];
  /** Firefox's preferences. */
  readonly firefoxUserPrefs?: LaunchOptions["firefoxUserPrefs"];
}

/**
 * Launches this run's browser for the app in `app` (its folder): Firefox in a data folder of its
 * own there (`firefoxEnv`), with `extras` for the engine they name.
 */
export function launchBrowser(app: URL, extras: LaunchExtras = {}): Promise<Browser> {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  if (BROWSER === "firefox") {
    const prefs = extras.firefoxUserPrefs === undefined ? {} : { firefoxUserPrefs: extras.firefoxUserPrefs };
    return type.launch({ env: firefoxEnv(app), ...prefs });
  }
  if (BROWSER === "chromium" && extras.chromiumArgs !== undefined)
    return type.launch({ args: [...extras.chromiumArgs] });
  return type.launch();
}
