// Settings (S07) as the user meets it: every section the build has, each with no serious or
// critical axe violation in the dialog, Graphics on Linux only, and theme, accent and page
// appearance shared across windows and ticked in the View menu. The session runs check that
// choices made here survive a relaunch (session-save, session-restore).
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { platform } from "node:process";
import { $, browser, expect } from "@wdio/globals";
import { focusWindow, menuClick, waitForMenus } from "../app.ts";

const AXE = readFileSync(createRequire(import.meta.url).resolve("axe-core/axe.min.js"), "utf8");
const SECTIONS = ["Files", "Appearance and accessibility", "Updates", "Privacy", "About"];
const LINUX = platform === "linux";

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

/** Chooses Dark, Plum and Sepia in one window and verifies a second window and the View menu match. */
async function sharesAppearanceAcrossWindows(): Promise<void> {
  const main = await browser.getWindowHandle();
  const before = await browser.getWindowHandles();
  try {
    await focusWindow();
    await menuClick("cmd:view.theme=dark");
    await menuClick("cmd:view.accent=plum");
    await menuClick("cmd:view.page-appearance=sepia");
    await browser.waitUntil(
      async () => {
        const cur = await rootAppearance();
        return cur.theme === "dark" && cur.accent === "plum" && cur.page === "sepia";
      },
      { timeoutMsg: "the first window didn't apply the view settings" },
    );
    const menus = await waitForMenus(/Dark\s+✓\s+<cmd:view\.theme=dark>/, "View ▸ Theme didn't tick Dark");
    expect(menus).toMatch(/Plum\s+✓\s+<cmd:view\.accent=plum>/);
    expect(menus).toMatch(/Sepia\s+✓\s+<cmd:view\.page-appearance=sepia>/);
    expect(menus).toMatch(/Single page.*\(disabled\)/);

    await menuClick("cmd:window.new");
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length > before.length, {
      timeoutMsg: "New window opened none",
    });
    const added = (await browser.getWindowHandles()).find((handle) => !before.includes(handle)) ?? "";
    await browser.switchToWindow(added);
    await browser.waitUntil(
      async () => {
        const cur = await rootAppearance();
        return cur.theme === "dark" && cur.accent === "plum" && cur.page === "sepia";
      },
      { timeoutMsg: "the second window didn't pick up the theme, accent and page appearance" },
    );
    await menuClick("cmd:view.theme=light");
    await browser.switchToWindow(main);
    await browser.waitUntil(async () => (await rootAppearance()).theme === "light", {
      timeoutMsg: "the first window didn't follow the second window's theme change",
    });
    await browser.switchToWindow(added);
    await browser.closeWindow();
    await browser.switchToWindow(main);
  } finally {
    await browser.switchToWindow(main);
    await menuClick("cmd:view.theme=system");
    await menuClick("cmd:view.accent=teal");
    await menuClick("cmd:view.page-appearance=original");
    await browser.waitUntil(
      async () => {
        const cur = await rootAppearance();
        return cur.theme === null && cur.accent === "teal" && cur.page === "original";
      },
      { timeoutMsg: "didn't restore the default appearance" },
    );
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
