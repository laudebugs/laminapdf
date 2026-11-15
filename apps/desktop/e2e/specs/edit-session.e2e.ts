// The edit session (E1h): View ▸ Rotate page right turns the shown page in the engine, which
// draws it from the edited file; Undo and Redo, from the Edit menu and from their keys, take the
// turn back and make it again, and the window's title has a "•" while the turn isn't saved.
// Closing a window with unsaved edits asks first; Cancel keeps it, and closing one with none
// doesn't ask. WebDriver can't reach the OS's menus: e2e_menu_click chooses an item as a click
// would.
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import { fixture, focusWindow, menuClick, openAndWait, waitForMenus } from "../app.ts";

const HELLO = fixture("fixtures", "golden", "examples", "hello.pdf");
const MOD = platform === "darwin" ? Key.Command : Key.Ctrl;
const CLEAN_TITLE = "hello.pdf — Lamina PDF";
const DIRTY_TITLE = "• hello.pdf — Lamina PDF";

/** Page 1's `/Rotate` as the engine reports it. */
async function rotation(): Promise<number | undefined> {
  return browser.execute(() => globalThis.__laminaE2e?.rotation(0));
}

/** Waits until page 1 is turned to `degrees` and the title says whether the turn is unsaved. */
async function turnedTo(degrees: number, why: string): Promise<void> {
  const title = degrees === 0 ? CLEAN_TITLE : DIRTY_TITLE;
  await browser.waitUntil(async () => (await rotation()) === degrees && (await browser.getTitle()) === title, {
    timeoutMsg: `${why}: page 1 is at ${String(await rotation())}°, the title "${await browser.getTitle()}"`,
  });
}

/** Opens a new window from the File menu, switches to it and opens `path` there; its handle. */
async function newWindowWith(path: string): Promise<string> {
  const before = await browser.getWindowHandles();
  await menuClick("cmd:window.new");
  await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
    timeoutMsg: "New window opened none",
  });
  const added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle)) ?? "";
  await browser.switchToWindow(added);
  await focusWindow();
  await openAndWait(path);
  return added;
}

/** The keys of Redo here: ⇧⌘Z on macOS, Ctrl+Y elsewhere. */
const REDO_KEYS = platform === "darwin" ? [Key.Command, Key.Shift, "z"] : [Key.Ctrl, "y"];

describe("the edit session", () => {
  it("turns a page, and undoes and redoes the turn from the Edit menu and the keys", async () => {
    await openAndWait(HELLO);
    await focusWindow();
    await waitForMenus(/Rotate page right {2}<cmd:pages\.rotate-right>/, "Rotate page right isn't enabled");
    await menuClick("cmd:pages.rotate-right");
    await turnedTo(90, "Rotate page right");
    await waitForMenus(/Undo \[\w+\+Z\] {2}<cmd:edit\.undo>/, "Undo isn't enabled after the turn");
    await browser.keys([MOD, "z"]);
    await turnedTo(0, "the undo keys");
    await menuClick("cmd:edit.redo");
    await turnedTo(90, "Edit ▸ Redo");
    await menuClick("cmd:edit.undo");
    await turnedTo(0, "Edit ▸ Undo");
    await browser.keys(REDO_KEYS);
    await turnedTo(90, "the redo keys");
  });

  it("asks before closing a window with unsaved edits, and Cancel keeps it", async () => {
    // From the turn the test before left.
    const handles = await browser.getWindowHandles();
    await menuClick("cmd:window.close");
    const question = $("[role=alertdialog]");
    await question.waitForDisplayed({ timeoutMsg: "closing the window didn't ask" });
    expect(await question.getText()).toContain("Close this window without saving?");
    await $("button=Cancel").click();
    await question.waitForDisplayed({ reverse: true });
    expect(await browser.getWindowHandles()).toEqual(handles);
    expect(await browser.getTitle()).toBe(DIRTY_TITLE);
    // Undone, the document matches its file again: nothing is lost by closing it.
    await browser.keys([MOD, "z"]);
    await turnedTo(0, "the undo before the clean close");
  });

  it("closes a window with no unsaved edits without asking, and one with them once told to", async () => {
    const main = await browser.getWindowHandle();
    const clean = await newWindowWith(HELLO);
    await menuClick("cmd:window.close");
    await browser.waitUntil(async () => !(await browser.getWindowHandles()).includes(clean), {
      timeoutMsg: "the window with no edits didn't close at once",
    });
    await browser.switchToWindow(main);
    const edited = await newWindowWith(HELLO);
    await menuClick("cmd:pages.rotate-right");
    await turnedTo(90, "Rotate page right in the new window");
    await menuClick("cmd:window.close");
    await $("[role=alertdialog]").waitForDisplayed({ timeoutMsg: "closing the edited window didn't ask" });
    await $("button=Close without saving").click();
    await browser.waitUntil(async () => !(await browser.getWindowHandles()).includes(edited), {
      timeoutMsg: "Close without saving didn't close the window",
    });
    await browser.switchToWindow(main);
    await focusWindow();
  });
});
