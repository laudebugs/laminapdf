// The built reader against axe (todo C7a): no serious or critical violation in any main state
// (home, an open document with its sidebar, find, View settings, Settings, Keyboard shortcuts, the
// palette) in each UI theme, in this run's browser. With motion reduced in Settings, nothing in the
// chrome animates.

import { env } from "node:process";
import axe from "axe-core";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { appVersion } from "../plugins/app-version.ts";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, load, type Watched } from "./reader.ts";

const origin = inject("origin");

/** Stores `appearance` before the app runs, as Settings would have. */
const withAppearance = (appearance: object) => ({
  script: (arg: number[]) => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    localStorage.setItem("lamina.appearance", String.fromCharCode(...arg));
  },
  arg: [...JSON.stringify(appearance)].map((c) => c.charCodeAt(0)),
});

/** axe's serious and critical violations on the page now, as `id: targets`, without the rules `off`. */
async function serious(page: Page, off: readonly string[] = []): Promise<string[]> {
  await page.evaluate(axe.source);
  return page.evaluate(async (off) => {
    const run = (window as unknown as { axe: typeof axe }).axe;
    const result = await run.run(document, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
      rules: Object.fromEntries(off.map((id) => [id, { enabled: false }])),
    });
    return result.violations
      .filter((v) => v.impact === "serious" || v.impact === "critical")
      .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  }, off);
}

describe(`accessibility in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader in `theme`, with axe allowed in past the CSP. */
  const start = (theme: string, extra: object = {}): Promise<Watched> =>
    load(browser, origin, "/", withAppearance({ theme, ...extra }), {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
      bypassCSP: true,
    });

  it.each(["light", "dark", "high-contrast"])("passes axe in every main state, %s theme", async (theme) => {
    const watched = await start(theme);
    const { page } = watched;
    const found: Record<string, string[]> = {};
    found.home = await serious(page);
    await chooseFile(watched, "outline.pdf");
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    found.open = await serious(page);
    await page.keyboard.press("ControlOrMeta+F");
    await page.getByRole("search").getByRole("textbox").waitFor();
    found.find = await serious(page);
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "View settings" }).click();
    await page.getByRole("dialog", { name: "View" }).waitFor();
    found.view = await serious(page);
    await page.getByRole("button", { name: "More settings…" }).click();
    await page.getByRole("dialog", { name: "Settings" }).waitFor();
    found.settings = await serious(page);
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Keyboard shortcuts" }).click();
    await page.getByRole("dialog", { name: "Keyboard shortcuts" }).waitFor();
    found.shortcuts = await serious(page);
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: /^Commands/ }).click();
    await page.getByRole("dialog").getByRole("combobox").waitFor();
    // A row's group and chip describe it, as in a menu; @laminapdf/ui's palette tests check that
    // description, which axe's experimental label-content-name-mismatch doesn't allow for.
    found.palette = await serious(page, ["label-content-name-mismatch"]);
    expect(found).toEqual({ home: [], open: [], find: [], view: [], settings: [], shortcuts: [], palette: [] });
    await page.close();
  });

  it("sets the theme and accent before the app's first paint, and changes them from View settings", async () => {
    const { page } = await start("dark", { accent: "plum" });
    const attrs = () =>
      page.evaluate(() => [document.documentElement.dataset.theme, document.documentElement.dataset.accent]);
    expect(await attrs()).toEqual(["dark", "plum"]);
    await page.getByRole("button", { name: "View settings" }).click();
    const view = page.getByRole("dialog", { name: "View" });
    // Each radio's label is what a pointer presses.
    await view.locator("label", { hasText: "Light" }).click();
    await view.locator('label[data-accent="blue"]').click();
    expect(await attrs()).toEqual(["light", "blue"]);
    const stored = await page.evaluate(() => JSON.parse(localStorage.getItem("lamina.appearance") ?? "{}"));
    expect([stored.theme, stored.accent]).toEqual(["light", "blue"]);
    await page.close();
  });

  it("shows the reader's version in Settings › About", async () => {
    const { page } = await start("light");
    // The version the build was given: a release tag's, else package.json's.
    const version = appVersion(env);
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: "Settings" }).click();
    const settings = page.getByRole("dialog", { name: "Settings" });
    await settings.getByRole("button", { name: "About" }).click();
    await expect.poll(() => settings.textContent()).toContain(`version ${version}`);
    await page.close();
  });

  it("runs no animation in the chrome with motion reduced in Settings", async () => {
    const watched = await start("light", { motion: "reduce" });
    const { page } = watched;
    await chooseFile(watched, "outline.pdf");
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    await page.getByRole("button", { name: "View settings" }).click();
    await page.getByRole("dialog", { name: "View" }).waitFor();
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: /^Commands/ }).click();
    await page.getByRole("dialog").getByRole("combobox").waitFor();
    const moving = await page.evaluate(() =>
      document.getAnimations().map((a) => (a.effect as KeyframeEffect | null)?.target?.className ?? "?"),
    );
    expect(moving).toEqual([]);
    const behaviour = await page.evaluate(
      () => getComputedStyle(document.querySelector(".lamina-viewer") as Element).scrollBehavior,
    );
    expect(behaviour).toBe("auto");
    await page.close();
  });
});
