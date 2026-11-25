// Settings (S07) as the user meets it: every section the build has, each with no serious or
// critical axe violation in the dialog, Graphics on Linux only, and theme, accent and page
// appearance shared across windows, reflected in the View popover and Settings, and ticked in the
// View menu. The session runs check that choices made here survive a relaunch (session-save,
// session-restore).
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { closeWindowFor, fixture, focusWindow, menuClick, menuClickAndGo, openAndWait, waitForMenus } from "../app.ts";

const AXE = readFileSync(createRequire(import.meta.url).resolve("axe-core/axe.min.js"), "utf8");
const SECTIONS = ["Files", "Appearance and accessibility", "Updates", "Privacy", "About"];
const LINUX = platform === "linux";
const EXAMPLE = fixture("fixtures", "golden", "examples", "hello.pdf");

/** The Settings dialog's sections, by the names its list shows. */
async function sections(): Promise<string[]> {
  return browser.execute(() =>
    [...globalThis.document.querySelectorAll(".paper-settings-nav-name")].map((name) => name.textContent ?? ""),
  );
}

/** The dialog's serious and critical axe violations, by rule. */
async function violations(): Promise<string[]> {
  await browser.execute(AXE);
  return browser.execute(async () => {
    const axe = (
      globalThis as unknown as {
        axe: { run(context: Element): Promise<{ violations: { id: string; impact: string }[] }> };
      }
    ).axe;
    const dialog = globalThis.document.querySelector('[role="dialog"]');
    if (dialog === null) return ["no dialog"];
    const results = await axe.run(dialog);
    return results.violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
  });
}

/** The appearance attributes set on this window's document root. */
async function rootAppearance(): Promise<{ theme: string | null; accent: string | null; page: string | null }> {
  return browser.execute(() => ({
    theme: globalThis.document.documentElement.getAttribute("data-theme"),
    accent: globalThis.document.documentElement.getAttribute("data-accent"),
    page: globalThis.document.documentElement.getAttribute("data-page-appearance"),
  }));
}

/** Waits until the active window's document root carries `wanted`. */
async function waitForAppearance(
  wanted: { theme: string | null; accent: string; page: string },
  timeoutMsg: string,
): Promise<void> {
  await browser.waitUntil(
    async () => {
      const cur = await rootAppearance();
      return cur.theme === wanted.theme && cur.accent === wanted.accent && cur.page === wanted.page;
    },
    { timeoutMsg },
  );
}

/**
 * Opens Settings and waits for every section. Each test opens it itself: wdio.conf.ts's
 * `beforeTest` closes everything before each test, after any `beforeEach` here.
 */
async function openSettings(): Promise<string[]> {
  await $('button[aria-label="More"]').click();
  await $('//*[@role="menuitem"][contains(., "Settings")]').click();
  // Updates, Privacy and About appear once the main process has answered.
  await browser.waitUntil(async () => (await sections()).length >= SECTIONS.length, {
    timeoutMsg: "Settings didn't list its sections",
  });
  return sections();
}

/** Chooses Dark, Plum and Sepia with a document open and verifies the View menu, popover and a second window match. */
async function sharesAppearanceAcrossWindows(): Promise<void> {
  const main = await browser.getWindowHandle();
  let added: string | undefined;
  try {
    await openAndWait(EXAMPLE);
    await focusWindow();
    await menuClick("cmd:view.theme=dark");
    await menuClick("cmd:view.accent=plum");
    await menuClick("cmd:view.page-appearance=sepia");
    await waitForAppearance(
      { theme: "dark", accent: "plum", page: "sepia" },
      "the first window didn't apply the view settings",
    );
    const menus = await waitForMenus(/Dark\s+✓\s+<cmd:view\.theme=dark>/, "View ▸ Theme didn't tick Dark");
    expect(menus).toMatch(/Plum\s+✓\s+<cmd:view\.accent=plum>/);
    expect(menus).toMatch(/Sepia\s+✓\s+<cmd:view\.page-appearance=sepia>/);
    expect(menus).toMatch(/Single page.*\(disabled\)/);

    // The View settings popover reflects the menu's choices.
    await $('button[aria-label="View settings"]').click();
    await expect($('button[aria-pressed="true"]*=Sepia')).toBeDisplayed();
    await browser.keys("Escape");

    const before = await browser.getWindowHandles();
    await menuClickAndGo("cmd:window.new");
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
      timeoutMsg: "New window opened none",
    });
    added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle));
    if (added === undefined) throw new Error("couldn't locate the new window handle");
    await browser.switchToWindow(added);
    await waitForAppearance(
      { theme: "dark", accent: "plum", page: "sepia" },
      "the second window didn't pick up the theme, accent and page appearance",
    );
    await focusWindow();
    await menuClick("cmd:view.theme=light");
    await browser.switchToWindow(main);
    await browser.waitUntil(async () => (await rootAppearance()).theme === "light", {
      timeoutMsg: "the first window didn't follow the second window's theme change",
    });
  } finally {
    for (const handle of await browser.getWindowHandles()) {
      if (handle === main) continue;
      await browser.switchToWindow(handle);
      await closeWindowFor(main).catch(() => undefined);
    }
    await browser.switchToWindow(main);
    await focusWindow();
    await menuClick("cmd:view.theme=system");
    await menuClick("cmd:view.accent=teal");
    await menuClick("cmd:view.page-appearance=original");
    await waitForAppearance({ theme: null, accent: "teal", page: "original" }, "didn't restore the default appearance");
  }
}

describe("Settings", () => {
  afterEach(async () => {
    await browser.keys("Escape");
  });

  it("should list every section, and Graphics only on Linux", async () => {
    const expected = LINUX ? [...SECTIONS, "Graphics"] : SECTIONS;
    expect((await openSettings()).sort()).toEqual([...expected].sort());
  });

  it("should have no serious accessibility problem in any section", async () => {
    const found: Record<string, string[]> = {};
    const names = await openSettings();
    for (const name of names) {
      await $(`//button[.//span[@class="paper-settings-nav-name" and text()=${JSON.stringify(name)}]]`).click();
      const problems = await violations();
      if (problems.length > 0) found[name] = problems;
    }
    // Every section checked, so an empty list can't pass.
    expect([names.length >= SECTIONS.length, found]).toEqual([true, {}]);
  });

  it(
    "should share theme, accent and page appearance with a second window and tick the View menu",
    sharesAppearanceAcrossWindows,
  );
});
