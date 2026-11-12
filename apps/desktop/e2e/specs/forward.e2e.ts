// Opening from the OS: a second launch with a path, as a double-click or "Open with" starts one on Windows
// and Linux, hands the file to the running app and exits; the running app shows it. The file's
// name has a space and non-ASCII letters. (macOS sends Finder's files as an event instead:
// e2e/MANUAL-OPEN.md.)
import { expect } from "@wdio/globals";
import { inkedPixels, launchAgain, waitForDocument } from "../app.ts";
import { oddlyNamedCopy } from "../paths.ts";

describe("opening from a second launch", () => {
  it("should show the file in the running app, and the second process should exit", async () => {
    const code = await launchAgain([oddlyNamedCopy()]);
    expect(code).toBe(0);
    await waitForDocument(2);
    expect(await inkedPixels(0)).toBeGreaterThan(0);
  });
});
