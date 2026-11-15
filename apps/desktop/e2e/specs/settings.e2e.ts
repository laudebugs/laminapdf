// Settings (S07) as the user meets it: every section the build has, each with no serious or
// critical axe violation in the dialog, and Graphics on Linux only, where its WebKitGTK workarounds
// apply. The session runs check that a choice made here survives a relaunch (session-save,
// session-restore).
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { platform } from "node:process";
import { browser, expect } from "@wdio/globals";
import { closeAll } from "../app.ts";

const AXE = readFileSync(createRequire(import.meta.url).resolve("axe-core/axe.min.js"), "utf8");
const SECTIONS = ["Files", "Appearance", "Updates", "Privacy", "About"];
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

describe("Settings", () => {
  before(async () => {
    await closeAll();
    await $('button[aria-label="More"]').click();
    await $('//*[@role="menuitem"][contains(., "Settings")]').click();
    await $(".paper-settings-nav-name").waitForExist();
  });

  after(async () => {
    await browser.keys("Escape");
  });

  it("should list every section, and Graphics only on Linux", async () => {
    const expected = LINUX ? [...SECTIONS, "Graphics"] : SECTIONS;
    expect(new Set(await sections())).toEqual(new Set(expected));
  });

  it("should have no serious accessibility problem in any section", async () => {
    const found: Record<string, string[]> = {};
    for (const name of await sections()) {
      await $(`//button[.//span[@class="paper-settings-nav-name" and text()=${JSON.stringify(name)}]]`).click();
      const problems = await violations();
      if (problems.length > 0) found[name] = problems;
    }
    expect(found).toEqual({});
  });
});
