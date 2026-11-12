// Where the e2e run finds the repository, the app's e2e build and its own output.
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { platform } from "node:process";
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
 * A copy of the two-page invoice fixture named with a space and non-ASCII letters, as a user's
 * files are, in the run's output folder.
 */
export function oddlyNamedCopy(): string {
  const copy = join(output, "Relevé de compte 2026.pdf");
  mkdirSync(output, { recursive: true });
  copyFileSync(join(root, "fixtures", "golden", "examples", "invoice_from_json.pdf"), copy);
  return copy;
}
