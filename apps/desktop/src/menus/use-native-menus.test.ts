import { type CommandContext, createWebPlatform, ShellLayout } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import { networkActions } from "../network.tsx";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { desktopRegistry } from "./menus.ts";
import { menuState, runMenuCommand } from "./use-native-menus.tsx";

/** The desktop's context with `actions`, and a reader with no document open. */
function context(actions: CommandContext["actions"] = {}, checked?: CommandContext["checked"]): CommandContext {
  return {
    platform: createDesktopPlatform(createWebPlatform()),
    actions,
    ...(checked === undefined ? {} : { checked }),
    reader: {
      getState: () => ({ phase: "idle" }) as never,
      subscribe: () => () => undefined,
      openFromPicker: () => Promise.resolve(),
      submitPassword: () => Promise.resolve(),
    },
  };
}

describe("menuState", () => {
  it("enables what the window can run now: commands it handles, and none that needs a document", () => {
    const state = menuState(
      desktopRegistry(),
      context({ "window.new": () => undefined, "file.print": () => undefined, ...networkActions(new ShellLayout()) }),
    );
    const enabled = (id: string) => state.find((s) => s.id === id)?.enabled;
    expect(enabled("window.new")).toBe(true);
    expect(enabled("file.open")).toBe(true);
    expect(enabled("privacy.network-activity")).toBe(true);
    // Print needs a document; Find has no handler in this window.
    expect(enabled("file.print")).toBe(false);
    expect(enabled("find.open")).toBe(false);
  });

  it("enables theme, accent and page appearance with their current choice, while page layout stays disabled", () => {
    const state = menuState(
      desktopRegistry(),
      context(
        {
          "view.theme": () => undefined,
          "view.accent": () => undefined,
          "view.page-appearance": () => undefined,
        },
        {
          "view.theme": "dark",
          "view.accent": "plum",
          "view.page-appearance": "sepia",
        },
      ),
    );
    const entry = (id: string) => state.find((s) => s.id === id);
    expect(entry("view.theme")).toEqual({ id: "view.theme", enabled: true, checked: "dark" });
    expect(entry("view.accent")).toEqual({ id: "view.accent", enabled: true, checked: "plum" });
    expect(entry("view.page-appearance")).toEqual({ id: "view.page-appearance", enabled: true, checked: "sepia" });
    // Page layout has no handler yet, so its submenu stays disabled.
    expect(entry("view.page-layout")?.enabled).toBe(false);
  });
});

describe("runMenuCommand", () => {
  it("runs view choice commands through the registry with the menu's argument", async () => {
    const theme = vi.fn();
    const accent = vi.fn();
    const pageAppearance = vi.fn();
    const ctx = context({
      "view.theme": theme,
      "view.accent": accent,
      "view.page-appearance": pageAppearance,
    });
    const registry = desktopRegistry();
    runMenuCommand(registry, ctx, { id: "view.theme", arg: "dark" });
    runMenuCommand(registry, ctx, { id: "view.accent", arg: "green" });
    runMenuCommand(registry, ctx, { id: "view.page-appearance", arg: "high-contrast" });
    await vi.waitFor(() => {
      expect(theme).toHaveBeenCalledWith("dark");
      expect(accent).toHaveBeenCalledWith("green");
      expect(pageAppearance).toHaveBeenCalledWith("high-contrast");
    });
  });

  it("hands an action that isn't a command its argument: Move tab to window ▸", () => {
    const move = vi.fn();
    runMenuCommand(desktopRegistry(), context({ "tab.move-to-window": move }), {
      id: "tab.move-to-window",
      arg: "doc-2",
    });
    expect(move).toHaveBeenCalledWith("doc-2");
  });

  it("does nothing for a command the window can't run now", async () => {
    const print = vi.fn();
    runMenuCommand(desktopRegistry(), context({ "file.print": print }), { id: "file.print" });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(print).not.toHaveBeenCalled();
  });
});
