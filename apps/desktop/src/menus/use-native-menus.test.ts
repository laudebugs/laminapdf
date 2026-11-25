import { type CommandContext, createWebPlatform, type ViewerControls } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { desktopRegistry } from "./menus.ts";
import { menuState, runMenuCommand } from "./use-native-menus.tsx";

/** The desktop's context with `actions`, and a reader with no document open unless `open` is set. */
function context(
  actions: CommandContext["actions"] = {},
  options: { readonly open?: boolean; readonly viewer?: ViewerControls } = {},
): CommandContext {
  const phase = options.open === true ? "open" : "idle";
  return {
    platform: createDesktopPlatform(createWebPlatform()),
    actions,
    ...(options.viewer === undefined ? {} : { viewer: options.viewer }),
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

  it("enables nav.back and nav.forward only when the viewer's history has an entry on that side", async () => {
    let page = 0;
    const pages = [0];
    let index = 0;
    const history = {
      getState: () => ({ back: pages[index - 1], forward: pages[index + 1] }),
      subscribe: () => () => undefined,
      back: () => {
        if (index <= 0) return false;
        index -= 1;
        page = pages[index] ?? 0;
        return true;
      },
      forward: () => {
        if (index >= pages.length - 1) return false;
        index += 1;
        page = pages[index] ?? 0;
        return true;
      },
    };
    const viewer: ViewerControls = {
      getState: () => ({ pageCount: 10, currentPage: page, zoom: "fit-width", percent: 100 }),
      subscribe: () => () => undefined,
      goToPage: (target) => {
        page = target;
      },
      jumpToPage: (target) => {
        pages.splice(index + 1, pages.length, target);
        index = pages.length - 1;
        page = target;
      },
      setZoom: () => undefined,
      rotate: () => undefined,
      history,
    };
    const registry = desktopRegistry();
    const ctx = context(
      {
        "nav.back": () => {
          history.back();
        },
        "nav.forward": () => {
          history.forward();
        },
      },
      { open: true, viewer },
    );
    const enabled = (id: string) => menuState(registry, ctx).find((s) => s.id === id)?.enabled;

    expect([enabled("nav.back"), enabled("nav.forward")]).toEqual([false, false]);

    viewer.jumpToPage(4);
    expect([enabled("nav.back"), enabled("nav.forward")]).toEqual([true, false]);

    runMenuCommand(registry, ctx, { id: "nav.back" });
    await vi.waitFor(() => expect(page).toBe(0));
    expect([enabled("nav.back"), enabled("nav.forward")]).toEqual([false, true]);

    runMenuCommand(registry, ctx, { id: "nav.forward" });
    await vi.waitFor(() => expect(page).toBe(4));
    expect([enabled("nav.back"), enabled("nav.forward")]).toEqual([true, false]);
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
