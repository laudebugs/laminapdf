// Tabs and windows: each file opens in a tab, a file opened again shows its tab, a tab
// comes back where it was, a tab moves to a new window with its page and zoom (an encrypted one
// without asking for its password again), and closing every tab leaves the start screen.
import { $, browser, expect } from "@wdio/globals";
import {
  closeAll,
  drawsAfterZoomOut,
  fixture,
  focusWindow,
  goToPage,
  moveToNewWindow,
  open,
  openAndWait,
  shownTab,
  tabNames,
  view,
  waitForSharp,
  waitForShown,
  withText,
  zoomOnce,
} from "../app.ts";

const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** The shown document's page count, 0 while the window shows none (a new window, before its tab). */
function pagesShown(): Promise<number> {
  return browser.execute(() => globalThis.__laminaE2e?.view()?.pageCount ?? 0);
}

/** The window the app started with, where every scenario before this one ran. */
let home = "";

/**
 * Closes the window WebDriver is on and goes back to `handle`, once the closed one is gone: until
 * then it can still hold the keyboard, and a shortcut sent next goes to it.
 */
async function closeWindowFor(handle: string): Promise<void> {
  const closing = await browser.getWindowHandle();
  await browser.closeWindow();
  await browser.waitUntil(async () => !(await browser.getWindowHandles()).includes(closing), {
    timeoutMsg: "the window didn't close",
  });
  await browser.switchToWindow(handle);
  // macOS can leave it in the background, where WebKit slows its work (a shortcut, an open).
  await focusWindow();
}

/**
 * Leaves the window the scenarios began in, alone and on its start screen. The window tests start
 * here so that each, and the retry of a failed attempt, opens its own files: an attempt that
 * failed leaves windows and documents open (an encrypted one unlocked), and a retry built on them
 * fails at once, hiding the first attempt's error behind its own.
 */
async function startOver(): Promise<void> {
  for (const handle of await browser.getWindowHandles()) {
    if (handle === home) continue;
    await browser.switchToWindow(handle);
    await closeWindowFor(home);
  }
  await browser.switchToWindow(home);
  await closeAll();
}

/** Moves flowing_report.pdf, shown on its page 5, to a new window, then invoice.pdf to that window from the strip's menu. */
async function movesToWindows(): Promise<void> {
  await startOver();
  await openAndWait(example("hello.pdf"));
  await openAndWait(example("invoice.pdf"));
  await openAndWait(example("flowing_report.pdf"));
  await goToPage(5);
  await waitForSharp(4);
  await zoomOnce("Zoom in");
  const { currentPage, percent } = await view();
  const main = await moveToNewWindow();
  await browser.waitUntil(async () => (await pagesShown()) === 13, { timeoutMsg: "the tab didn't arrive" });
  await browser.waitUntil(async () => (await view()).currentPage === currentPage, { timeoutMsg: "a different page" });
  expect((await view()).percent).toBe(percent);
  expect(await browser.getTitle()).toBe("flowing_report.pdf — Lamina PDF");
  const other = await browser.getWindowHandle();
  await browser.switchToWindow(main);
  // It left this window, which shows the next tab.
  await browser.waitUntil(async () => (await tabNames()).join() === "hello.pdf,invoice.pdf", {
    timeoutMsg: "the moved tab is still in this window",
  });
  // Move tab to window ▸ the new one, from the strip's menu.
  await $('[role="tab"]=invoice.pdf').click();
  await waitForShown("invoice.pdf");
  await zoomOnce("Zoom in");
  const invoiceZoom = (await view()).percent;
  await $('button[aria-label="All tabs, 2 open"]').click();
  await $('[role="menuitem"]=Move tab to flowing_report.pdf — Lamina PDF').click();
  await browser.switchToWindow(other);
  await browser.waitUntil(async () => (await shownTab()) === "invoice.pdf", {
    timeoutMsg: "invoice.pdf didn't arrive",
  });
  expect(await tabNames()).toEqual(["flowing_report.pdf", "invoice.pdf"]);
  expect((await view()).percent).toBe(invoiceZoom);
  await closeWindowFor(main);
}

/** Opens the encrypted fixture, moves it to a new window, then on to another, and closes the one between. */
async function movesEncrypted(): Promise<void> {
  await startOver();
  await open(fixture("fixtures", "encrypted", "r3-rc4-128-user.pdf"));
  await $('input[aria-label="Password"]').setValue("user-pw");
  await $("button=Open").click();
  await browser.waitUntil(async () => (await pagesShown()) > 0, { timeoutMsg: "it didn't open" });
  const main = await moveToNewWindow();
  await browser.waitUntil(async () => (await pagesShown()) > 0, { timeoutMsg: "the tab didn't arrive" });
  await waitForSharp(0);
  await expect($('input[aria-label="Password"]')).not.toBeExisting();
  // Moved on again, then the window it came from closes: it stays unlocked and keeps drawing.
  const source = await moveToNewWindow();
  const last = await browser.getWindowHandle();
  await browser.switchToWindow(source);
  await closeWindowFor(last);
  await browser.waitUntil(async () => (await pagesShown()) > 0, { timeoutMsg: "the tab didn't arrive" });
  await drawsAfterZoomOut();
  await expect($('input[aria-label="Password"]')).not.toBeExisting();
  await closeWindowFor(main);
}

describe("tabs and windows", () => {
  before(async () => {
    home = await browser.getWindowHandle();
  });

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
    await movesToWindows();
  });

  it("should move an encrypted document without asking for its password again", async () => {
    await movesEncrypted();
  });

  it("should go back to the start screen when the last tab closes", async () => {
    await closeAll();
    await expect(withText("Choose a file")).toBeDisplayed();
    expect(await browser.getTitle()).toBe("Lamina PDF");
  });
});
