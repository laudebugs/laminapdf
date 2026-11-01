// The Firefox workaround every package's browser config uses.
import { mkdirSync } from "node:fs";
import { env } from "node:process";
import { fileURLToPath } from "node:url";
import { playwright } from "@vitest/browser-playwright";

/**
 * Firefox (Playwright's build, 155 and later) reads the user's Firefox data folder even with its
 * own profile, and fails with "Could not find profile folder." where it can't (macOS refuses it
 * on the self-hosted runners). MOZ_APP_DATA and MOZ_LOCAL_APP_DATA point it at a folder of its
 * own, in the package's node_modules (so runners sharing a machine don't share it).
 */
export function firefoxProvider(packageDir: URL) {
  const data = fileURLToPath(new URL("node_modules/.cache/lamina-firefox/", packageDir));
  mkdirSync(data, { recursive: true });
  return playwright({ launchOptions: { env: { ...env, MOZ_APP_DATA: data, MOZ_LOCAL_APP_DATA: data } } });
}
