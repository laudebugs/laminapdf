// The last session, second run (D4d): the app relaunched after session-save brings back both
// windows, the main one at its size, their tabs, and the moved tab at its page and zoom; a tab not
// shown opens when it is first shown. Turning "Reopen last session" off deletes the session file.
import { existsSync, readFileSync } from "node:fs";
import { $, browser, expect } from "@wdio/globals";
import { shownTab, tabNames, toggleSetting, view, waitForSharp, waitForShown } from "../app.ts";
import { sessionFile } from "../paths.ts";
import { type Expected, expected, MAIN_TABS, OTHER_TABS } from "../session.ts";

const measured = () => JSON.parse(readFileSync(expected, "utf8")) as Expected;

describe("the last session after a relaunch", () => {
  it("should bring back both windows and the main window's tabs", async () => {
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === 2, {
      timeoutMsg: "the second window didn't come back",
    });
    // WebDriver may start on either window: the main one is the one with the main tabs.
    await browser.waitUntil(
      async () => {
        for (const handle of await browser.getWindowHandles()) {
          await browser.switchToWindow(handle);
          if (JSON.stringify(await tabNames()) === JSON.stringify(MAIN_TABS)) return true;
        }
        return false;
      },
      { timeoutMsg: "the main window's tabs didn't come back" },
    );
    await waitForShown("hello.pdf");
  });

  it("should bring back the main window's size", async () => {
    const { width, height } = await browser.getWindowSize();
    const want = measured();
    const screen = await browser.execute(() => [window.screen.availWidth, window.screen.availHeight]);
    // The sizes in the failure, so a runner's screen that can't hold the window shows as such.
    expect({ width, height, screen }).toEqual({
      width: Math.abs(width - want.width) <= 2 ? width : want.width,
      height: Math.abs(height - want.height) <= 2 ? height : want.height,
      screen,
    });
  });

  it("should show the other window's tab at its page and zoom", async () => {
    const main = await browser.getWindowHandle();
    const other = (await browser.getWindowHandles()).find((handle) => handle !== main) ?? main;
    await browser.switchToWindow(other);
    await waitForShown(OTHER_TABS[0] ?? "");
    await waitForSharp(1);
    const shown = await view();
    expect([shown.currentPage, shown.percent]).toEqual([1, measured().percent]);
    await browser.switchToWindow(main);
  });

  it("should open a tab not shown when it is first shown", async () => {
    await $('//*[@role="tab"][contains(., "invoice_from_json.pdf")]').click();
    await waitForShown("invoice_from_json.pdf");
    await browser.waitUntil(async () => (await view()).pageCount === 2, { timeoutMsg: "the tab didn't open" });
    expect(await shownTab()).toBe("invoice_from_json.pdf");
  });

  it("should delete the session file when Reopen last session is turned off", async () => {
    await toggleSetting("Reopen last session");
    await browser.waitUntil(() => !existsSync(sessionFile), { timeoutMsg: "session.json is still there" });
  });
});
