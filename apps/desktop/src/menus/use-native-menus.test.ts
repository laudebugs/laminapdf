import { type CommandContext, createWebPlatform, ModeRegistry, ShellLayout, shellActions } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { desktopRegistry } from "./menus.ts";
import { menuState, runMenuCommand } from "./use-native-menus.tsx";

/** The desktop's context with `actions` and optional `layout` / open phase. */
function context(
  actions: CommandContext["actions"] = {},
  { phase = "empty", layout }: { phase?: "empty" | "open"; layout?: ShellLayout } = {},
): CommandContext {
  return {
    platform: createDesktopPlatform(createWebPlatform()),
    actions,
    ...(layout === undefined ? {} : { layout }),
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

  it("enables Focus mode with a document open and ticks it while on", async () => {
    const layout = new ShellLayout();
    const actions = shellActions(layout, new ModeRegistry(), { rail: false });
    const ctx = context(actions, { phase: "open", layout });
    const registry = desktopRegistry();
    expect(menuState(registry, ctx).find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: true,
      checked: false,
    });
    runMenuCommand(registry, ctx, { id: "view.focus-mode" });
    await vi.waitFor(() => expect(layout.getState().focus).toBe(true));
    expect(menuState(registry, ctx).find((s) => s.id === "view.focus-mode")).toEqual({
      id: "view.focus-mode",
      enabled: true,
      checked: true,
    });
  });
});

describe("runMenuCommand", () => {
  it("runs a command through the registry with the menu's argument", async () => {
    const theme = vi.fn();
    runMenuCommand(desktopRegistry(), context({ "view.theme": theme }), { id: "view.theme", arg: "dark" });
    await vi.waitFor(() => expect(theme).toHaveBeenCalledWith("dark"));
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
