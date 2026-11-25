// The desktop chrome as a window rather than a web page (motion plan §5): the hand on its controls
// and links (the grab hand on its tabs), chrome labels that can't be selected while the document can, no webview context menu
// on the chrome, a tab strip that is there from the first tab so a second moves nothing below it,
// and Settings › Motion turning reduced motion on in the window.
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { Key } from "webdriverio";
import { fixture, openAndWait } from "../app.ts";

const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** `property`'s computed value on the first element matching `selector`, or "" without one. */
function computed(selector: string, property: string): Promise<string> {
  return browser.execute(
    (at, name) => {
      const element = globalThis.document.querySelector(at);
      return element === null ? "" : getComputedStyle(element).getPropertyValue(name);
    },
    selector,
    property,
  );
}

/** Whether selection is off on `selector`: `user-select`, or WebKit's prefixed name. */
async function unselectable(selector: string): Promise<boolean> {
  const values = [await computed(selector, "user-select"), await computed(selector, "-webkit-user-select")];
  return values.includes("none");
}

/** Whether a right-click on `selector` was refused the webview's menu (the event's default prevented). */
function menuRefused(selector: string): Promise<boolean> {
  return browser.execute((at) => {
    const element = globalThis.document.querySelector(at);
    if (element === null) throw new Error(`no ${at}`);
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true });
    element.dispatchEvent(event);
    return event.defaultPrevented;
  }, selector);
}

/** The document's canvas's top edge, in CSS pixels. */
function canvasTop(): Promise<number> {
  return browser.execute(() => globalThis.document.querySelector(".desktop-viewer")?.getBoundingClientRect().top ?? -1);
}

describe("the desktop chrome", () => {
  it("should show the hand on the start screen's button", async () => {
    await $("button=Choose a file").waitForDisplayed();
    const cursor = await browser.execute(
      () =>
        getComputedStyle(
          [...globalThis.document.querySelectorAll("button")].find((b) => b.textContent === "Choose a file") as Element,
        ).cursor,
    );
    expect(cursor).toBe("pointer");
  });

  it("should open the strip with the first tab, and a second tab should move nothing below it", async () => {
    await openAndWait(example("hello.pdf"));
    await $('[role="tablist"]').waitForExist({ timeoutMsg: "no tab strip with one tab" });
    const top = await canvasTop();
    await openAndWait(example("invoice.pdf"));
    expect(await canvasTop()).toBe(top);
  });

  it("should show the grab hand on tabs, the hand on the strip's menu, and keep the chrome's labels unselectable", async () => {
    // A tab can be dragged (to reorder it, or out to another window), so it shows the grab cursor.
    expect(await computed('[role="tab"]', "cursor")).toBe("grab");
    expect(await computed(".paper-document-tabs-menu", "cursor")).toBe("pointer");
    expect(await unselectable(".paper-top-bar")).toBe(true);
    expect(await unselectable('[role="tab"]')).toBe(true);
    expect(await unselectable(".desktop-viewer")).toBe(false);
  });

  it("should refuse the webview's context menu on the chrome, and keep it on the document", async () => {
    expect(await menuRefused(".paper-top-bar")).toBe(true);
    expect(await menuRefused(".paper-reader-status")).toBe(true);
    expect(await menuRefused(".desktop-viewer")).toBe(false);
  });

  it("should reduce motion in the window when Settings › Motion says so", async () => {
    const motion = () => browser.execute(() => globalThis.document.documentElement.getAttribute("data-motion"));
    const motionGroup = '//*[@role="radiogroup" and .//*[@class="paper-field-label" and text()="Motion"]]';
    await browser.keys([platform === "darwin" ? Key.Command : Key.Ctrl, ","]);
    try {
      // The sections list's entry, then the Motion radio group's choices (Theme also has "Match system").
      await $('//*[normalize-space(text())="Appearance and accessibility"]').click();
      await $(`${motionGroup}//span[@class="paper-choice-label" and text()="Reduce motion"]`).click();
      await browser.waitUntil(async () => (await motion()) === "reduce", {
        timeoutMsg: "the window didn't reduce motion",
      });
      await $(`${motionGroup}//span[@class="paper-choice-label" and text()="Match system"]`).click();
      await browser.waitUntil(async () => (await motion()) === null, { timeoutMsg: "the window kept reduced motion" });
    } finally {
      // Settings opens where it was left: back at Files, where the scenarios after this one look.
      await $('//*[normalize-space(text())="Files"]')
        .click()
        .catch(() => undefined);
      await browser.keys("Escape");
    }
  });
});
