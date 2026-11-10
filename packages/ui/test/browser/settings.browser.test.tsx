// The Settings dialog on a phone, where it opens as a bottom sheet: one column, the sections as a
// row of buttons above the chosen section's fields, and nothing reaching past the sheet's edge
// (docs/bugs/settings-cut-off-on-phones).

import { type ReactNode, useState } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { page } from "vitest/browser";
import {
  AppearanceSettings,
  DEFAULT_APPEARANCE,
  SettingsDialog,
  SettingsRegistry,
  useSettingsSection,
} from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

beforeEach(async () => {
  await page.viewport(360, 740);
});

afterEach(async () => {
  cleanup();
  await page.viewport(1280, 800);
});

/** Settings with the reader's two sections: appearance, and a storage section of plain text. */
function Settings(): ReactNode {
  const [appearance, setAppearance] = useState(DEFAULT_APPEARANCE);
  const [registry] = useState(() => new SettingsRegistry());
  useSettingsSection(registry, "appearance", "Appearance and accessibility", () => (
    <AppearanceSettings appearance={appearance} onChange={setAppearance} />
  ));
  useSettingsSection(registry, "storage", "Storage and privacy", () => (
    <p className="paper-field-help">Recent files stay on this device.</p>
  ));
  return <SettingsDialog registry={registry} isOpen onOpenChange={() => {}} />;
}

async function openSettings(): Promise<HTMLElement> {
  renderIn(<Settings />, { density: "touch" });
  await expect.poll(() => document.querySelector(".paper-settings")).not.toBeNull();
  return document.querySelector<HTMLElement>(".paper-settings") as HTMLElement;
}

describe("Settings on a phone", () => {
  it("opens as a sheet whose options all fit inside it", async () => {
    const settings = await openSettings();
    expect(settings.closest(".paper-sheet")).not.toBeNull();
    const edge = settings.getBoundingClientRect().right;
    const overflowing = [...settings.querySelectorAll<HTMLElement>("label, button, h3, p")]
      .filter((el) => el.getBoundingClientRect().right > edge + 0.5)
      .map((el) => el.textContent?.trim());
    expect(overflowing).toEqual([]);
  });

  it("puts the sections in a row above the fields, the fields at the sheet's full width", async () => {
    const settings = await openSettings();
    const nav = settings.querySelector<HTMLElement>(".paper-settings-nav") as HTMLElement;
    const section = settings.querySelector<HTMLElement>(".paper-settings-section") as HTMLElement;
    const buttons = [...nav.querySelectorAll<HTMLElement>("button")].map((b) => b.getBoundingClientRect());
    expect([
      nav.getBoundingClientRect().bottom <= section.getBoundingClientRect().top,
      new Set(buttons.map((b) => Math.round(b.top))).size,
      Math.round(section.getBoundingClientRect().width) === Math.round(settings.getBoundingClientRect().width),
    ]).toEqual([true, 1, true]);
  });
});
