// Tabs and windows (todo D4a): each file opens in a tab, a file opened again shows its tab, a tab
// comes back where it was, a tab moves to a new window with its page and zoom (an encrypted one
// without asking for its password again), and closing every tab leaves the start screen.
import { $, browser, expect } from "@wdio/globals";
import {
  closeAll,
  fixture,
  goToPage,
  open,
  openAndWait,
  runCommand,
  shownTab,
  tabNames,
  view,
  waitForSharp,
  withText,
  zoomOnce,
} from "../app.ts";

const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** Waits until the strip shows `name` as the shown tab. */
async function waitForShown(name: string): Promise<void> {
  await browser.waitUntil(async () => (await shownTab()) === name, { timeoutMsg: `${name} isn't the shown tab` });
}

/** Moves the shown tab to a new window and switches to it; resolves to this window's handle. */
async function moveToNewWindow(): Promise<string> {
  const here = await browser.getWindowHandle();
  const before = await browser.getWindowHandles();
  await runCommand("Move tab to new window");
  await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
    timeoutMsg: "no new window",
  });
  const added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle));
  if (added === undefined) throw new Error("no new window handle");
  await browser.switchToWindow(added);
  return here;
}

/** Closes the window WebDriver is on and goes back to `handle`. */
async function closeWindowFor(handle: string): Promise<void> {
  await browser.closeWindow();
  await browser.switchToWindow(handle);
}

describe("tabs and windows", () => {
  it("should open each file in its own tab, and show a file opened again in its tab", async () => {
    await openAndWait(example("hello.pdf"));
    await openAndWait(example("invoice.pdf"));
    await openAndWait(example("flowing_report.pdf"));
    expect(await tabNames()).toEqual(["hello.pdf", "invoice.pdf", "flowing_report.pdf"]);
    expect(await browser.getTitle()).toBe("flowing_report.pdf — Lamina PDF");
    await open(example("hello.pdf"));
    await waitForShown("hello.pdf");
    expect(await tabNames()).toHaveLength(3);
  });

  it("should show a tab again at the page it was on", async () => {
    await $('[role="tab"]=flowing_report.pdf').click();
    await waitForShown("flowing_report.pdf");
    await goToPage(5);
    await waitForSharp(4);
    await $('[role="tab"]=invoice.pdf').click();
    await waitForShown("invoice.pdf");
    await $('[role="tab"]=flowing_report.pdf').click();
    await waitForShown("flowing_report.pdf");
    await browser.waitUntil(async () => (await view()).currentPage === 4, { timeoutMsg: "not back on page 5" });
  });

  it("should move a tab to a new window at its page and zoom", async () => {
    await zoomOnce("Zoom in");
    const { currentPage, percent } = await view();
    const main = await moveToNewWindow();
    await browser.waitUntil(async () => (await view()).pageCount === 13, { timeoutMsg: "the tab didn't arrive" });
    await browser.waitUntil(async () => (await view()).currentPage === currentPage, { timeoutMsg: "a different page" });
    expect((await view()).percent).toBe(percent);
    expect(await browser.getTitle()).toBe("flowing_report.pdf — Lamina PDF");
    await closeWindowFor(main);
    // It left this window, which shows the next tab.
    expect(await tabNames()).toEqual(["hello.pdf", "invoice.pdf"]);
  });

  it("should move an encrypted document without asking for its password again", async () => {
    await open(fixture("fixtures", "encrypted", "r3-rc4-128-user.pdf"));
    await $('input[aria-label="Password"]').setValue("user-pw");
    await $("button=Open").click();
    await waitForShown("r3-rc4-128-user.pdf");
    const main = await moveToNewWindow();
    await browser.waitUntil(async () => (await view()).pageCount > 0, { timeoutMsg: "the tab didn't arrive" });
    await waitForSharp(0);
    await expect($('input[aria-label="Password"]')).not.toBeExisting();
    await closeWindowFor(main);
  });

  it("should go back to the start screen when the last tab closes", async () => {
    await closeAll();
    await expect(withText("Choose a file")).toBeDisplayed();
    expect(await browser.getTitle()).toBe("Lamina PDF");
  });
});
