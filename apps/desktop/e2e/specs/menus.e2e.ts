// The native menus: built from the shared shortcut map, they show the focused window's
// state (Print and Close tab disabled with nothing open), run their commands in that window, and a
// shortcut a menu also binds acts once. Open recent lists the recent files and opens one (Clear menu
// runs in the cold-open run); the Window menu lists the windows and moves a tab to another; ⌘, or Ctrl+, opens Settings.
// WebDriver can't reach the OS's menus: e2e_menu_click chooses an item as a click would, and
// e2e_menu_snapshot reads them.
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  closeAll,
  fixture,
  focusWindow,
  menuClick,
  openAndWait,
  tabNames,
  view,
  waitForMenus,
  withText,
} from "../app.ts";

const example = (name: string) => fixture("fixtures", "golden", "examples", name);
const MOD = platform === "darwin" ? Key.Command : Key.Ctrl;
/** The menus' name for the Mod key on this platform. */
const MOD_NAME = platform === "darwin" ? "Cmd" : "Ctrl";

/** With every tab closed: Print and Close tab disabled, Open enabled with its keys. */
async function disabledWithNothingOpen(): Promise<void> {
  await closeAll();
  await focusWindow();
  const menus = await waitForMenus(/Print… \[\w+\+P\] \(disabled\)/, "Print isn't disabled");
  expect(menus).toContain(`Open a file… [${MOD_NAME}+O]  <cmd:file.open>`);
  expect(menus).toMatch(/Close tab \[\w+\+W\] \(disabled\)/);
}

/** One press of the zoom-in keys is one step; View ▸ Zoom in is another. */
async function zoomsOnce(): Promise<void> {
  await openAndWait(example("hello.pdf"));
  await focusWindow();
  await waitForMenus(/Zoom in \[\w+\+Equal\] {2}<cmd:view.zoom-in>/, "Zoom in isn't enabled");
  const before = (await view()).percent;
  await browser.keys([MOD, "="]);
  await browser.waitUntil(async () => (await view()).percent !== before, { timeoutMsg: "the keys didn't zoom" });
  const once = (await view()).percent;
  // An echo through the menu's accelerator would take a second step.
  await browser.pause(600);
  expect((await view()).percent).toBe(once);
  await menuClick("cmd:view.zoom-in");
  await browser.waitUntil(async () => (await view()).percent > once, { timeoutMsg: "the menu didn't zoom" });
}

/**
 * Open recent lists hello.pdf (opened before) and opens it with no dialog. Clear menu runs in
 * the cold-open run (cold-open.e2e.ts), which reads the list this run leaves.
 */
async function opensRecent(): Promise<void> {
  const listed = await waitForMenus(/hello\.pdf {2}<recent:[^>]+>/, "hello.pdf isn't in Open recent");
  const id = /hello\.pdf {2}<recent:([^>]+)>/.exec(listed)?.[1] ?? "";
  await closeAll();
  await menuClick(`recent:${id}`);
  await browser.waitUntil(async () => (await browser.getTitle()) === "hello.pdf — Lamina PDF", {
    timeoutMsg: "Open recent didn't open hello.pdf",
  });
}

/** New window from the File menu, listed in the Window menu; the first window's tab moves to it. */
async function movesFromWindowMenu(): Promise<void> {
  await closeAll();
  await openAndWait(example("hello.pdf"));
  await openAndWait(example("invoice.pdf"));
  const main = await browser.getWindowHandle();
  const before = await browser.getWindowHandles();
  await menuClick("cmd:window.new");
  await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
    timeoutMsg: "New window opened none",
  });
  const added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle)) ?? "";
  const menus = await waitForMenus(/<window:doc-\d+>/, "the Window menu doesn't list the new window");
  const label = /<window:(doc-\d+)>/.exec(menus)?.[1] ?? "";
  // From the new window, the Window menu brings the first one forward.
  await browser.switchToWindow(added);
  await menuClick("window:main");
  await waitForMenus(/✓ {2}<window:main>/, "the first window didn't come forward");
  // There, its shown tab (invoice.pdf) moves to the new window.
  await browser.switchToWindow(main);
  await menuClick(`move:${label}`);
  await browser.waitUntil(async () => (await tabNames()).join() !== "hello.pdf,invoice.pdf", {
    timeoutMsg: "invoice.pdf didn't leave this window",
  });
  await browser.switchToWindow(added);
  await browser.waitUntil(async () => (await browser.getTitle()) === "invoice.pdf — Lamina PDF", {
    timeoutMsg: "invoice.pdf didn't arrive",
  });
  await browser.closeWindow();
  await browser.switchToWindow(main);
}

describe("the native menus", () => {
  it("should disable what needs a document while none is open", disabledWithNothingOpen);
  it("should zoom one step for one press of the zoom-in keys, and again from the View menu", zoomsOnce);
  it("should list recent files in Open recent and open one", opensRecent);
  it("should list the windows in the Window menu and move a tab to another from it", movesFromWindowMenu);

  it("should open Settings with Mod+,", async () => {
    await browser.keys([MOD, ","]);
    await expect(withText("Remember recent files")).toBeDisplayed();
    await browser.keys("Escape");
    await expect($('[role="dialog"]')).not.toBeExisting();
  });
});
