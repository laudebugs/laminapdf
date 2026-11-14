// The last session, first run (D4d): two windows with their tabs, a page and a zoom, and the main
// window's size, kept in session.json even after "Clear recent", which empties only the recent
// list. The next run (session-restore) relaunches the app and checks what comes back.
import { writeFileSync } from "node:fs";
import { browser, expect } from "@wdio/globals";
import { closeAll, goToPage, moveToNewWindow, openAndWait, runCommand, view, zoomOnce } from "../app.ts";
import { type Expected, example, expected, MAIN_TABS, OTHER_TABS, savedTabs } from "../session.ts";

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
});
