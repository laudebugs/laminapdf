// Where the e2e run finds the repository, the app's e2e build and its own output.
import { copyFileSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { env, platform } from "node:process";
import { fileURLToPath } from "node:url";

/** The repository's root. */
export const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..");

/** The e2e build of the app (`desktop:build:e2e`). */
export const appBinary = join(
  root,
  "target",
  "debug",
  platform === "win32" ? "laminapdf-desktop.exe" : "laminapdf-desktop",
);

/** Screenshots and logs of failed scenarios, which CI uploads. */
export const output = join(root, "target", "desktop-e2e");

/**
 * Copies that scenarios change, replace or delete. They sit under `target/fixtures`, because the
 * e2e build's file pick (`e2e_queue_pick`) opens only files under the repository's fixtures.
 */
export const workingCopies = join(root, "target", "fixtures", "desktop-e2e");

/**
 * A copy of the two-page invoice fixture named with a space and non-ASCII letters, as a user's
 * files are, in the run's output folder.
 */
export function oddlyNamedCopy(): string {
  const copy = join(output, "Relevé de compte 2026.pdf");
  mkdirSync(output, { recursive: true });
  copyFileSync(join(root, "fixtures", "golden", "examples", "invoice_from_json.pdf"), copy);
  return copy;
}

/** The e2e build's config folder: Tauri's `app_config_dir` for its identifier, on each OS. */
const configDir = join(
  platform === "darwin"
    ? join(homedir(), "Library", "Application Support")
    : platform === "win32"
      ? (env.APPDATA ?? join(homedir(), "AppData", "Roaming"))
      : (env.XDG_CONFIG_HOME ?? join(homedir(), ".config")),
  "com.laminapdf.desktop.e2e",
);

/** The last session, as the app writes it (src-tauri/src/session/mod.rs). */
export const sessionFile = join(configDir, "session.json");

/** The app's preferences (src-tauri/src/prefs.rs). */
export const preferencesFile = join(configDir, "preferences.json");
