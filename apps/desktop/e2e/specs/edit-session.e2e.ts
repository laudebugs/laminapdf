// The edit session: View ▸ Rotate page right turns the shown page in the engine, which
// draws it from the edited file; Undo and Redo, from the Edit menu and from their keys, take the
// turn back and make it again, and the window's title has a "•" while the turn isn't saved.
// Closing a window with unsaved edits asks first; Cancel keeps it, and closing one with none
// doesn't ask. WebDriver can't reach the OS's menus: e2e_menu_click chooses an item as a click
// would.
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import {
  AFTER_THE_ANSWER_MS,
  closeWindowFor,
  fixture,
  focusWindow,
  menuClick,
  menuClickAndGo,
  now,
  openAndWait,
  queuePick,
  waitForMenus,
  waitForSharpSince,
} from "../app.ts";

const HELLO = fixture("fixtures", "golden", "examples", "hello.pdf");
/** The new windows' file: one open in another window would show there, in its tab. */
const INVOICE = fixture("fixtures", "golden", "examples", "invoice.pdf");
const MOD = platform === "darwin" ? Key.Command : Key.Ctrl;

/** Page 1's `/Rotate` as the engine reports it. */
async function rotation(): Promise<number | undefined> {
  return browser.execute(() => globalThis.__laminaE2e?.rotation(0));
}

/** Whether page 1 shows wider than tall: hello.pdf's portrait page after a quarter turn. */
async function shownLandscape(): Promise<boolean> {
  return browser.execute(() => {
    const box = globalThis.document.querySelector('[data-page-index="0"]')?.getBoundingClientRect();
    return box !== undefined && box.width > box.height;
  });
}

/**
 * Runs `step`, then waits until page 1 is turned to `degrees` in the engine, shows that way and is
 * drawn sharp again, and the title says whether the turn is unsaved.
 */
async function turnsTo(degrees: number, why: string, step: () => Promise<unknown>, file = "hello.pdf"): Promise<void> {
  const since = await now();
  await step();
  const title = `${degrees === 0 ? "" : "• "}${file} — Lamina PDF`;
  const landscape = degrees % 180 !== 0;
  await browser.waitUntil(
    async () =>
      (await rotation()) === degrees && (await shownLandscape()) === landscape && (await browser.getTitle()) === title,
    { timeoutMsg: `${why}: page 1 didn't show turned to ${degrees}° with the title "${title}"` },
  );
  await waitForSharpSince(0, since);
}

/**
 * Opens a new window from the File menu, switches to it and opens `path` there with Choose a file;
 * its handle. The page clicks the button itself: under WebKitGTK and xvfb, WebDriver's click
 * sometimes never reached a window opened after another one closed (the page's own calls did),
 * and these tests are about closing windows, not about the open.
 */
async function newWindowWith(path: string): Promise<string> {
  const before = await browser.getWindowHandles();
  await menuClickAndGo("cmd:window.new");
  await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
    timeoutMsg: "New window opened none",
  });
  const added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle)) ?? "";
  await browser.switchToWindow(added);
  await focusWindow();
  await $("button=Choose a file").waitForExist({ timeoutMsg: "the new window shows no Choose a file" });
  await queuePick(path);
  const since = await now();
  await browser.execute(() => {
    const buttons = Array.from(globalThis.document.querySelectorAll("button"));
    buttons.find((button) => button.textContent === "Choose a file")?.click();
  });
  await waitForSharpSince(0, since);
  return added;
}

/** The keys of Redo here: ⇧⌘Z on macOS, Ctrl+Y elsewhere. */
const REDO_KEYS = platform === "darwin" ? [Key.Command, Key.Shift, "z"] : [Key.Ctrl, "y"];

/**
 * After each test, closes the windows a test left open and puts WebDriver back on the window the
 * spec started in, focused; gives that window's handle. A test that fails between opening and
 * closing a window leaves WebDriver on a window it may have closed, and the window it opened still
 * open: its retry and the specs after it would start from no window, or beside a stray one. Not
 * `startOver`, which would also close the main window's document that the next test goes on with.
 */
function returnToTheMainWindow(): () => string {
  let main = "";
  before(async () => {
    main = await browser.getWindowHandle();
  });
  afterEach(async () => {
    const handles = await browser.getWindowHandles();
    if (!handles.includes(main)) return;
    for (const handle of handles.filter((h) => h !== main)) {
      await browser.switchToWindow(handle);
      // A window with unsaved edits asks first and stays: the cleanup must not fail the run on it.
      await closeWindowFor(main).catch(() => undefined);
    }
    await browser.switchToWindow(main);
    await focusWindow();
  });
  return () => main;
}

describe("the edit session", () => {
  const mainWindow = returnToTheMainWindow();

  it("turns a page, and undoes and redoes the turn from the Edit menu and the keys", async () => {
    await openAndWait(HELLO);
    await focusWindow();
    // Nothing to undo or redo yet.
    await waitForMenus(
      /Undo \[\w+\+Z\] \(disabled\)[^\n]*\n\s*Redo \[[^\]]+\] \(disabled\)/,
      "Undo and Redo aren't disabled",
    );
    await waitForMenus(/Rotate page right {2}<cmd:pages\.rotate-right>/, "Rotate page right isn't enabled");
    await turnsTo(90, "Rotate page right", () => menuClick("cmd:pages.rotate-right"));
    await waitForMenus(
      /Undo \[\w+\+Z\] {2}<cmd:edit\.undo>\n\s*Redo \[[^\]]+\] \(disabled\)/,
      "after the turn, Undo isn't enabled or Redo isn't disabled",
    );
    await turnsTo(0, "the undo keys", () => browser.keys([MOD, "z"]));
    await waitForMenus(/Redo \[[^\]]+\] {2}<cmd:edit\.redo>/, "Redo isn't enabled after the undo");
    await turnsTo(90, "Edit ▸ Redo", () => menuClick("cmd:edit.redo"));
    await turnsTo(0, "Edit ▸ Undo", () => menuClick("cmd:edit.undo"));
    await turnsTo(90, "the redo keys", () => browser.keys(REDO_KEYS));
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
    expect(await browser.getTitle()).toBe("• hello.pdf — Lamina PDF");
    // Undone, the document matches its file again: nothing is lost by closing it.
    await turnsTo(0, "the undo before the clean close", () => browser.keys([MOD, "z"]));
  });

  it("closes a window with no unsaved edits without asking, and one with them once told to", async () => {
    const main = mainWindow();
    const clean = await newWindowWith(INVOICE);
    await menuClickAndGo("cmd:window.close");
    await browser.waitUntil(async () => !(await browser.getWindowHandles()).includes(clean), {
      timeoutMsg: "the window with no edits didn't close at once",
    });
    await browser.switchToWindow(main);
    const edited = await newWindowWith(INVOICE);
    await turnsTo(90, "Rotate page right in the new window", () => menuClick("cmd:pages.rotate-right"), "invoice.pdf");
    await menuClick("cmd:window.close");
    await $("[role=alertdialog]").waitForDisplayed({ timeoutMsg: "closing the edited window didn't ask" });
    // The click closes the window it is in: the page makes it after this call has answered.
    await browser.execute((wait) => {
      setTimeout(() => {
        const buttons = Array.from(globalThis.document.querySelectorAll("button"));
        buttons.find((button) => button.textContent === "Close without saving")?.click();
      }, wait);
    }, AFTER_THE_ANSWER_MS);
    await browser.waitUntil(async () => !(await browser.getWindowHandles()).includes(edited), {
      timeoutMsg: "Close without saving didn't close the window",
    });
    await browser.switchToWindow(main);
    await focusWindow();
  });
});
