// The last session, first run (D4d): two windows with their tabs, a page and a zoom, and the main
// window's size, kept in session.json even after "Clear recent", which empties only the recent
// list, and a setting turned off in Settings. The next run (session-restore) relaunches the app and
// checks what comes back.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { browser, expect } from "@wdio/globals";
import {
  closeAll,
  focusWindow,
  goToPage,
  menuClick,
  moveToNewWindow,
  openAndWait,
  runCommand,
  settingOn,
  toggleSetting,
  view,
  zoomOnce,
} from "../app.ts";
import { preferencesFile } from "../paths.ts";
import { type Expected, example, expected, MAIN_TABS, OTHER_TABS, savedTabs } from "../session.ts";

/** The setting the relaunch checks (session-restore). */
const RELOAD = "Reload files that change on disk";

describe("the session as the app runs", () => {
  it("should be kept with both windows, their tabs, a page and a zoom, after Clear recent", async () => {
    await closeAll();
    await openAndWait(example("invoice_from_json.pdf"));
    await openAndWait(example("markdown_to_pdf.pdf"));
    await goToPage(2);
    await zoomOnce("Zoom in");
    const { percent } = await view();
    const main = await moveToNewWindow();
    await browser.switchToWindow(main);
    await openAndWait(example("hello.pdf"));
    // Small enough for CI's screens (681 px high on macOS, 720 on Windows): the OS shrinks a
    // window taller than its screen when it comes back, which is no loss of the size.
    await browser.setWindowSize(900, 560);
    const { width, height } = await browser.getWindowSize();
    const measured: Expected = { percent, width, height };
    writeFileSync(expected, JSON.stringify(measured));
    await runCommand("Clear recent");
    await browser.waitUntil(() => JSON.stringify(savedTabs()) === JSON.stringify([MAIN_TABS, OTHER_TABS]), {
      timeout: 15_000,
      timeoutMsg: `the session file holds ${JSON.stringify(savedTabs())}`,
    });
    expect(savedTabs()).toEqual([MAIN_TABS, OTHER_TABS]);
  });

  it("should keep a setting changed in Settings, for the next launch to find", async () => {
    if (await settingOn(RELOAD)) await toggleSetting(RELOAD);
    expect(await settingOn(RELOAD)).toBe(false);
    await focusWindow();
    await menuClick("cmd:view.theme=dark");
    await browser.waitUntil(
      async () => {
        const applied = await browser.execute(() => globalThis.document.documentElement.getAttribute("data-theme"));
        const saved = existsSync(preferencesFile)
          ? (JSON.parse(readFileSync(preferencesFile, "utf8")) as { theme?: string }).theme
          : undefined;
        return applied === "dark" && saved === "dark";
      },
      { timeoutMsg: "Dark theme didn't apply and persist to preferences.json before relaunch" },
    );
  });
});
