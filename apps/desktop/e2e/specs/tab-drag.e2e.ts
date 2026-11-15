// Dragging tabs with the pointer: across the strip it reorders them; released off the strip it
// moves the tab to the window under the cursor, or to a new window when the cursor is outside
// every window. WebDriver's pointer moves stay inside the page and never move the OS's cursor, so
// `e2e_place_cursor` puts the cursor that the main process reads (src-tauri/src/tab_drag.rs).
import { $, browser, expect } from "@wdio/globals";
import {
  closeWindowFor,
  fixture,
  focusWindow,
  invoke,
  openAndWait,
  startOver,
  tabNames,
  waitForShown,
} from "../app.ts";

const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** The window the app started with. */
let home = "";

/** Where tab `name` is in the page, in CSS pixels. */
async function tabBox(name: string): Promise<{ x: number; y: number; width: number; height: number }> {
  return browser.execute((title) => {
    const tab = [...globalThis.document.querySelectorAll('[role="tab"]')].find((t) => t.textContent === title);
    const box = tab?.getBoundingClientRect();
    if (box === undefined) throw new Error(`no tab ${title}`);
    return { x: box.left, y: box.top, width: box.width, height: box.height };
  }, name);
}

/** Presses tab `name`, moves the pointer to `to` (in the page) in two steps, and lets go. */
async function dragTab(name: string, to: { x: number; y: number }): Promise<void> {
  const box = await tabBox(name);
  const from = { x: Math.round(box.x + box.width / 2), y: Math.round(box.y + box.height / 2) };
  await browser
    .action("pointer", { parameters: { pointerType: "mouse" } })
    .move({ origin: "viewport", ...from })
    .down({ button: 0 })
    .move({ origin: "viewport", x: from.x + 10, y: from.y })
    .move({ origin: "viewport", x: Math.round(to.x), y: Math.round(to.y) })
    .up({ button: 0 })
    .perform();
}

/** Where the main process finds the cursor when a drag ends: over window `label`, or outside them all. */
async function placeCursor(at: `window:${string}` | "outside" | "os"): Promise<void> {
  await invoke("e2e_place_cursor", { at });
}

/** A point in this window's page below the tab strip. */
async function belowTheStrip(): Promise<{ x: number; y: number }> {
  const box = await tabBox((await tabNames())[0] ?? "");
  return { x: box.x + box.width / 2, y: box.y + box.height + 200 };
}

describe("dragging tabs", () => {
  before(async () => {
    home = await browser.getWindowHandle();
  });

  afterEach(async () => {
    await placeCursor("os");
  });

  it("should reorder the tabs when one is dropped on another's first half", async () => {
    await startOver(home);
    await openAndWait(example("hello.pdf"));
    await openAndWait(example("invoice.pdf"));
    await openAndWait(example("flowing_report.pdf"));
    const onto = await tabBox("hello.pdf");
    await dragTab("flowing_report.pdf", { x: onto.x + 6, y: onto.y + onto.height / 2 });
    await browser.waitUntil(async () => (await tabNames()).join() === "flowing_report.pdf,hello.pdf,invoice.pdf", {
      timeoutMsg: "the tabs weren't reordered",
    });
    // The release that ended the drag clicked the tab too: it didn't change the shown one.
    await waitForShown("flowing_report.pdf");
  });

  it("should move a tab to the window it is dropped on, and to a new window when dropped outside them all", async () => {
    await startOver(home);
    await openAndWait(example("hello.pdf"));
    await openAndWait(example("invoice.pdf"));
    await openAndWait(example("flowing_report.pdf"));
    const before = await browser.getWindowHandles();
    // flowing_report.pdf out to a new window: the cursor is outside every window.
    await placeCursor("outside");
    await dragTab("flowing_report.pdf", await belowTheStrip());
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length === before.length + 1, {
      timeoutMsg: "no new window",
    });
    // The new window takes the focus, and WebDriver may follow it: the checks below are this window's.
    await browser.switchToWindow(home);
    await focusWindow();
    await browser.waitUntil(async () => (await tabNames()).join() === "hello.pdf,invoice.pdf", {
      timeoutMsg: "the tab is still in this window",
    });
    const other = (await browser.getWindowHandles()).find((handle) => !before.includes(handle)) ?? "";
    const label = ((await invoke("window_list")) as { label: string }[])[0]?.label ?? "";
    // invoice.pdf onto that window: the cursor is over it.
    await placeCursor(`window:${label}`);
    await dragTab("invoice.pdf", await belowTheStrip());
    await browser.switchToWindow(other);
    await browser.waitUntil(async () => (await tabNames()).join() === "flowing_report.pdf,invoice.pdf", {
      timeoutMsg: "invoice.pdf didn't arrive",
    });
    await expect($('[role="tab"][aria-selected="true"]')).toHaveText("invoice.pdf");
    await closeWindowFor(home);
  });
});
