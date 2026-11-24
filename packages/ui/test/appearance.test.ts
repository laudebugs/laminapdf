// The appearance's stored form and the attributes it sets, and the Settings dialog's sections.

import { describe, expect, it, vi } from "vitest";
import { registerReaderCommands } from "../src/commands/reader-commands.js";
import { CommandRegistry } from "../src/commands/registry.js";
import { Announcer } from "../src/reader/announcer.tsx";
import { applyAppearance, DEFAULT_APPEARANCE, parseAppearance } from "../src/reader/appearance.tsx";
import { applyPageAppearance, type ViewerPageAppearance } from "../src/reader/page-appearance.tsx";
import {
  choosePageAppearance,
  isPageAppearanceName,
  parsePageAppearanceChoices,
  resolvePageAppearance,
  undoPageAppearance,
} from "../src/reader/page-appearance-choices.ts";
import { SettingsRegistry } from "../src/reader/settings.tsx";

type Root = Parameters<typeof applyAppearance>[0];
type PageRoot = Parameters<typeof applyPageAppearance>[0];

/** An element's attributes, as `applyAppearance` sets them. */
function element(): Root & { attrs: Map<string, string> } {
  const attrs = new Map<string, string>();
  return {
    attrs,
    setAttribute: (name: string, value: string) => attrs.set(name, value),
    removeAttribute: (name: string) => attrs.delete(name),
    getAttribute: (name: string) => attrs.get(name) ?? null,
    // No window: no frames to suppress transitions across, so suppressMotion leaves it alone.
    ownerDocument: { defaultView: null },
  } as unknown as Root & { attrs: Map<string, string> };
}

describe("the appearance", () => {
  it("is the default with nothing stored, a damaged entry or unknown values", () => {
    for (const stored of [null, "", "{", "[]", "3", '{"theme":"sepia","accent":"red","motion":1,"singleKeys":"no"}']) {
      expect(parseAppearance(stored), String(stored)).toEqual(DEFAULT_APPEARANCE);
    }
  });

  it("keeps each known value it finds", () => {
    expect(parseAppearance('{"theme":"dark","accent":"plum","motion":"reduce","singleKeys":false}')).toEqual({
      theme: "dark",
      accent: "plum",
      motion: "reduce",
      singleKeys: false,
    });
  });

  it("sets data-theme, data-accent and data-motion, and leaves the theme to the system when asked", () => {
    const root = element();
    applyAppearance(root, { theme: "high-contrast", accent: "green", motion: "reduce", singleKeys: true });
    expect(Object.fromEntries(root.attrs)).toEqual({
      "data-theme": "high-contrast",
      "data-accent": "green",
      "data-motion": "reduce",
    });
    applyAppearance(root, DEFAULT_APPEARANCE);
    expect(Object.fromEntries(root.attrs)).toEqual({ "data-accent": "teal" });
  });
});

describe("the Settings dialog's sections", () => {
  it("keep their registration order, replace a section by id and remove it", () => {
    const registry = new SettingsRegistry();
    const listener = vi.fn();
    registry.subscribe(listener);
    const removeA = registry.register({ id: "appearance", title: "Appearance", render: () => null });
    registry.register({ id: "storage", title: "Storage", render: () => null });
    registry.register({ id: "appearance", title: "Look", render: () => null });
    expect(registry.sections().map((s) => [s.id, s.title])).toEqual([
      ["appearance", "Look"],
      ["storage", "Storage"],
    ]);
    // The first registration was replaced, so removing it leaves the replacement and tells no one.
    removeA();
    expect(registry.sections().map((s) => s.id)).toEqual(["appearance", "storage"]);
    expect(listener).toHaveBeenCalledTimes(3);
  });

  it("draw a section again on refresh, as a new object the dialog can't reuse, which its remover still removes", () => {
    const registry = new SettingsRegistry();
    const remove = registry.register({ id: "storage", title: "Storage", render: () => null });
    const before = registry.sections()[0];
    registry.refresh("storage");
    expect(registry.sections()[0]).not.toBe(before);
    expect(registry.sections()[0]?.title).toBe("Storage");
    remove();
    expect(registry.sections()).toEqual([]);
  });
});

describe("the announcer", () => {
  it("says the last polite message of a kind once its kind is quiet, and errors at once", () => {
    vi.useFakeTimers();
    const announcer = new Announcer(100);
    for (const page of [2, 3, 4]) announcer.polite(`Page ${page} of 9`, "page");
    announcer.polite("Zoom 150%", "zoom");
    expect(announcer.getState().polite).toBe("");
    announcer.assertive("This file is damaged");
    expect(announcer.getState().assertive).toBe("This file is damaged");
    vi.advanceTimersByTime(100);
    // Both kinds came due together: both are said.
    expect(announcer.getState().polite).toBe("Page 4 of 9. Zoom 150%");
    vi.advanceTimersByTime(1000);
    announcer.polite("Zoom 150%", "zoom");
    vi.advanceTimersByTime(100);
    expect(announcer.getState().polite).toBe("Zoom 150%");
    vi.advanceTimersByTime(1000);
    announcer.polite("Zoom 150%", "zoom");
    vi.advanceTimersByTime(100);
    // The same words again are changed so that they are read again.
    expect(announcer.getState().polite).toBe("Zoom 150%\u200b");
    vi.useRealTimers();
  });
});

describe("page appearance helpers", () => {
  it("recognises only known page appearance names", () => {
    expect(isPageAppearanceName("sepia")).toBe(true);
    expect(isPageAppearanceName("neon")).toBe(false);
  });

  it("drops unknown names when parsing stored page appearance choices", () => {
    expect(parsePageAppearanceChoices('{"all":"sepia","documents":{"a":"dark","b":"bogus"}}')).toEqual({
      all: "sepia",
      documents: { a: "dark" },
    });
  });

  it("falls back to high-contrast under forced colours when no choice is stored", () => {
    expect(resolvePageAppearance({ documents: {} }, undefined, true)).toBe("high-contrast");
    expect(resolvePageAppearance({ documents: {} }, undefined, false)).toBe("original");
  });

  it("chooses and undoes a document's page appearance", () => {
    const start = { stored: { documents: {} }, session: { documents: {} } };
    const doc = { key: "doc-1", kept: true };
    const next = choosePageAppearance(start.stored, start.session, doc, "sepia", "document");
    expect(resolvePageAppearance(next.stored, doc.key, false)).toBe("sepia");
    expect(undoPageAppearance(next, start, next, doc)).toEqual(start);
  });

  it("sets data-page-appearance on the root and forwards named modes to the viewer", () => {
    const root = {
      dataset: {} as Record<string, string>,
      ownerDocument: { defaultView: null },
    } as unknown as PageRoot;
    const seen: ViewerPageAppearance[] = [];
    const viewer = { setAppearance: (value: ViewerPageAppearance) => seen.push(value) };
    applyPageAppearance(root, viewer, "sepia");
    applyPageAppearance(root, viewer, "dark");
    expect(root.dataset.pageAppearance).toBe("dark");
    expect(seen).toEqual(["sepia", "dark"]);
  });

  it("reports checked state for delegated view commands from CommandContext.checked", () => {
    const registry = new CommandRegistry();
    registerReaderCommands(registry);
    const ctx = {
      actions: {
        "view.theme": () => undefined,
        "view.accent": () => undefined,
        "view.page-appearance": () => undefined,
      },
      checked: {
        "view.theme": "dark",
        "view.accent": "plum",
        "view.page-appearance": "sepia",
      },
    };
    expect(registry.availability("view.theme", ctx)).toEqual({ state: "available" });
    expect(registry.get("view.theme")?.checked?.(ctx)).toBe("dark");
    expect(registry.get("view.accent")?.checked?.(ctx)).toBe("plum");
    expect(registry.get("view.page-appearance")?.checked?.(ctx)).toBe("sepia");
  });
});
