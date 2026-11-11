// The Settings dialog on a phone, where it opens as a bottom sheet showing one screen at a time:
// the list of sections, each with what it is set to, then a section with "All settings" to go back;
// nothing reaches past the sheet's edge (docs/bugs/settings-cut-off-on-phones). Wider than a
// phone, the list stays beside the fields.

import { type ReactNode, useState } from "react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { page, userEvent } from "vitest/browser";
import {
  AppearanceSettings,
  appearanceSummary,
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
  useSettingsSection(
    registry,
    "appearance",
    "Appearance and accessibility",
    () => <AppearanceSettings appearance={appearance} onChange={setAppearance} />,
    { shows: [appearance], summary: () => appearanceSummary(appearance) },
  );
  useSettingsSection(registry, "storage", "Storage and privacy", () => (
    <p className="paper-field-help">Recent files stay on this device.</p>
  ));
  const [open, setOpen] = useState(true);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Open settings
      </button>
      <SettingsDialog registry={registry} isOpen={open} onOpenChange={setOpen} />
    </>
  );
}

async function openSettings(): Promise<HTMLElement> {
  renderIn(<Settings />, { density: "touch" });
  await expect.poll(() => document.querySelector(".paper-settings")).not.toBeNull();
  return document.querySelector<HTMLElement>(".paper-settings") as HTMLElement;
}

describe("Settings on a phone", () => {
  it("opens on the list of sections, each with what it is set to, and no fields", async () => {
    const settings = await openSettings();
    const rows = [...settings.querySelectorAll<HTMLElement>(".paper-settings-nav button")];
    expect([
      settings.closest(".paper-sheet") !== null,
      rows.map((row) => row.querySelector(".paper-settings-summary")?.textContent ?? null),
      visible(settings.querySelector(".paper-settings-section")),
    ]).toEqual([true, ["Match system · Teal · Shortcuts on", null], false]);
  });

  it("puts each name over its summary, a chevron at the end, and marks no row current", async () => {
    const settings = await openSettings();
    const first = row(settings, "Appearance and accessibility");
    const name = first.querySelector(".paper-settings-nav-name")?.getBoundingClientRect();
    const summary = first.querySelector(".paper-settings-summary")?.getBoundingClientRect();
    expect([
      (summary?.top ?? 0) >= (name?.bottom ?? 1),
      visible(first.querySelector(".paper-settings-chevron")),
      settings.querySelectorAll('[aria-current="page"]').length,
    ]).toEqual([true, true, 0]);
  });

  it("starts at the list again when Settings reopens", async () => {
    const settings = await openSettings();
    await userEvent.click(row(settings, "Storage and privacy"));
    await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector(".paper-settings")).toBeNull();
    await userEvent.click(page.getByRole("button", { name: "Open settings" }));
    await expect.poll(() => document.querySelector(".paper-settings")).not.toBeNull();
    const reopened = document.querySelector<HTMLElement>(".paper-settings") as HTMLElement;
    expect(visible(reopened.querySelector(".paper-settings-nav"))).toBe(true);
  });

  it("opens a section as its own screen, with its heading and focus on All settings", async () => {
    const settings = await openSettings();
    await userEvent.click(row(settings, "Appearance and accessibility"));
    expect([
      visible(settings.querySelector(".paper-settings-nav")),
      visible(settings.querySelector(".paper-settings-heading")),
      document.activeElement?.textContent,
    ]).toEqual([false, true, "All settings"]);
  });

  it("goes back to the list with focus on the section it came from", async () => {
    const settings = await openSettings();
    await userEvent.click(row(settings, "Storage and privacy"));
    await userEvent.click(page.getByRole("button", { name: "All settings" }));
    expect([
      visible(settings.querySelector(".paper-settings-section")),
      document.activeElement === row(settings, "Storage and privacy"),
    ]).toEqual([false, true]);
  });

  it("fits every option of a section inside the sheet", async () => {
    const settings = await openSettings();
    await userEvent.click(row(settings, "Appearance and accessibility"));
    const edge = settings.getBoundingClientRect().right;
    const overflowing = [...settings.querySelectorAll<HTMLElement>("label, button, h3, p")]
      .filter((el) => visible(el) && el.getBoundingClientRect().right > edge + 0.5)
      .map((el) => el.textContent?.trim());
    expect(overflowing).toEqual([]);
  });
});

describe("Settings wider than a phone", () => {
  beforeEach(async () => {
    await page.viewport(1280, 800);
  });

  it("keeps the list beside the fields, without summaries or a way back", async () => {
    renderIn(<Settings />);
    await expect.poll(() => document.querySelector(".paper-settings")).not.toBeNull();
    const settings = document.querySelector<HTMLElement>(".paper-settings") as HTMLElement;
    const nav = settings.querySelector<HTMLElement>(".paper-settings-nav") as HTMLElement;
    const section = settings.querySelector<HTMLElement>(".paper-settings-section") as HTMLElement;
    expect([
      nav.getBoundingClientRect().right <= section.getBoundingClientRect().left,
      visible(settings.querySelector(".paper-settings-summary")),
      visible(settings.querySelector(".paper-settings-chevron")),
      visible(settings.querySelector(".paper-settings-back")),
    ]).toEqual([true, false, false, false]);
  });

  it("keeps focus on the pressed section and marks it current", async () => {
    renderIn(<Settings />);
    await expect.poll(() => document.querySelector(".paper-settings")).not.toBeNull();
    const settings = document.querySelector<HTMLElement>(".paper-settings") as HTMLElement;
    const storage = row(settings, "Storage and privacy");
    await userEvent.click(storage);
    expect([document.activeElement === storage, storage.getAttribute("aria-current")]).toEqual([true, "page"]);
  });
});

/** Whether `el` is laid out (not hidden by the screen it belongs to). */
function visible(el: Element | null): boolean {
  return el !== null && el.getClientRects().length > 0;
}

/** The list's row for the section named `name`. */
function row(settings: HTMLElement, name: string): HTMLElement {
  const found = [...settings.querySelectorAll<HTMLElement>(".paper-settings-nav button")].find(
    (b) => b.querySelector(".paper-settings-nav-name")?.textContent === name,
  );
  if (found === undefined) throw new Error(`no row for ${name}`);
  return found;
}
