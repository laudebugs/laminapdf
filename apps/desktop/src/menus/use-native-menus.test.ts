import { type CommandContext, createWebPlatform, ModeRegistry, ShellLayout, shellActions } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { desktopRegistry } from "./menus.ts";
import { menuState, runMenuCommand } from "./use-native-menus.tsx";

/** The desktop's context with `actions` and optional `layout`, open phase and checked choices. */
function context(
  actions: CommandContext["actions"] = {},
  {
    phase = "empty",
    layout,
    checked,
  }: { phase?: "empty" | "open"; layout?: ShellLayout; checked?: CommandContext["checked"] } = {},
): CommandContext {
  return {
    platform: createDesktopPlatform(createWebPlatform()),
    actions,
    ...(layout === undefined ? {} : { layout }),
    ...(checked === undefined ? {} : { checked }),
    reader: {
      getState: () => ({ phase }) as never,
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
      context({ "window.new": () => undefined, "file.print": () => undefined }),
    );
    const enabled = (id: string) => state.find((s) => s.id === id)?.enabled;
    expect(enabled("window.new")).toBe(true);
    expect(enabled("file.open")).toBe(true);
    // Print needs a document; Find has no handler in this window.
    expect(enabled("file.print")).toBe(false);
    expect(enabled("find.open")).toBe(false);
  });

  it("keeps Focus mode disabled with no document open even when handled", () => {
    const layout = new ShellLayout();
    const actions = shellActions(layout, new ModeRegistry(), { rail: false });
    const state = menuState(desktopRegistry(), context(actions, { phase: "empty", layout }));
    expect(state.find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: false,
      checked: false,
    });
    expect(state.find((s) => s.id === "view.toggle-sidebar")?.enabled).toBe(false);
  });

  it("enables Focus mode with a document open, ticks it while on, and keeps the sidebar disabled", async () => {
    const layout = new ShellLayout();
    const actions = shellActions(layout, new ModeRegistry(), { rail: false });
    const ctx = context(actions, { phase: "open", layout });
    const registry = desktopRegistry();
    const before = menuState(registry, ctx);
    expect(before.find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: true,
      checked: false,
    });
    expect(before.find((s) => s.id === "view.toggle-sidebar")?.enabled).toBe(false);
    runMenuCommand(registry, ctx, { id: "view.focus-mode" });
    await vi.waitFor(() => expect(layout.getState().focus).toBe(true));
    expect(menuState(registry, ctx).find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: true,
      checked: true,
    });
    runMenuCommand(registry, ctx, { id: "view.focus-mode" });
    await vi.waitFor(() => expect(layout.getState().focus).toBe(false));
    expect(menuState(registry, ctx).find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: true,
      checked: false,
    });
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
        { checked: { "view.theme": "dark", "view.accent": "plum", "view.page-appearance": "sepia" } },
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
