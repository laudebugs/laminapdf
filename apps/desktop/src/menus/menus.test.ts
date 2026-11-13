import { type CommandContext, createWebPlatform, menuAccelerator, shortcutMap } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
import { MENU_LAYOUT } from "./layout.ts";
import { desktopRegistry, type MenusJson, menuCommandIds, nativeMenus } from "./menus.ts";

/** The desktop's context, as the commands read their titles and shortcuts in it. */
function desktopContext(): CommandContext {
  return { platform: createDesktopPlatform(createWebPlatform()) };
}

const menus = (): MenusJson => nativeMenus(desktopRegistry(), desktopContext());

/** Every command item, with its menu's platform folded into its own. */
function commandItems(): { id: string; mac?: string; other?: string; on?: string | undefined }[] {
  return menus().menus.flatMap((menu) =>
    menu.items.flatMap((item) => (item.kind === "command" ? [{ ...item, on: item.on ?? menu.on }] : [])),
  );
}

describe("the native menus' data", () => {
  it("takes every menu item's keys from the registry, as the shortcut help shows them", () => {
    const byId = new Map(
      desktopRegistry()
        .list()
        .map((c) => [c.id as string, c]),
    );
    for (const item of commandItems()) {
      const keys = byId.get(item.id)?.shortcuts ?? [];
      if (item.mac !== undefined) expect(keys.map((k) => k.mac)).toContain(item.mac);
      if (item.other !== undefined) expect(keys.map((k) => k.other)).toContain(item.other);
    }
  });

  it("gives each modified key to one item per platform", () => {
    for (const platform of ["mac", "other"] as const) {
      const owners = new Map<string, Set<string>>();
      for (const item of commandItems().filter((i) => i.on === undefined || i.on === platform)) {
        const keys = item[platform];
        if (keys !== undefined) owners.set(keys, (owners.get(keys) ?? new Set()).add(item.id));
      }
      const shared = [...owners].filter(([, ids]) => ids.size > 1).map(([keys]) => keys);
      expect(shared, platform).toEqual([]);
    }
  });

  it("lists the S03 shortcuts: Open, New window, Close tab and window, zoom, Settings, Quit off macOS", () => {
    const items = menus().menus.flatMap((m) => m.items.map((i) => ({ menu: m.id, ...i })));
    const keys = (id: string) => items.filter((i) => i.kind === "command" && i.id === id);
    expect(keys("file.open")[0]).toMatchObject({ mac: "Mod+O", other: "Mod+O" });
    expect(keys("window.new")[0]).toMatchObject({ mac: "Mod+Shift+N" });
    expect(keys("tab.close")[0]).toMatchObject({ mac: "Mod+W" });
    expect(keys("window.close")[0]).toMatchObject({ mac: "Mod+Shift+W" });
    expect(keys("view.zoom-in")[0]).toMatchObject({ mac: "Mod++", other: "Mod++" });
    expect(keys("app.quit")[0]).toMatchObject({ other: "Ctrl+Q", on: "other" });
    expect(keys("app.quit")[0]).not.toHaveProperty("mac");
    // Settings: the app menu on macOS, the end of Edit elsewhere, both with Mod+,.
    expect(keys("app.settings").map((i) => [i.menu, i.on])).toEqual([
      ["app", undefined],
      ["edit", "other"],
    ]);
  });

  it("shows the keys the shortcut help lists, and the help lists every key a menu shows", () => {
    // A window with a handler for every command the shell does: none is left out of the help.
    const registry = desktopRegistry();
    const actions = Object.fromEntries(registry.list().map((c) => [c.id, () => undefined]));
    const ctx: CommandContext = { ...desktopContext(), actions };
    const help = new Map(shortcutMap(registry, ctx).flatMap((g) => g.commands.map((c) => [c.id as string, c.keys])));
    const items = commandItems();
    for (const item of items) {
      if (item.mac !== undefined)
        expect(
          help.get(item.id)?.map((k) => k.mac),
          item.id,
        ).toContain(item.mac);
      if (item.other !== undefined)
        expect(
          help.get(item.id)?.map((k) => k.other),
          item.id,
        ).toContain(item.other);
    }
    // Every command the help lists with keys a menu can bind has a menu item showing them.
    const inMenus = new Set(items.map((i) => i.id));
    const missing = [...help.keys()].filter((id) => {
      const command = registry.get(id as never);
      const keys =
        command === undefined
          ? undefined
          : (menuAccelerator(command, ctx, "mac") ?? menuAccelerator(command, ctx, "other"));
      return keys !== undefined && !inMenus.has(id);
    });
    expect(missing).toEqual([]);
  });

  it("lists every command whose home is a menu", () => {
    const listed = new Set(
      MENU_LAYOUT.flatMap((m) =>
        m.entries.flatMap((e) => ("command" in e ? [e.command] : "choices" in e ? [e.choices] : [])),
      ),
    );
    // Open recent is a submenu of its files; Install is the web reader's.
    const elsewhere = new Set(["file.open-recent", "app.install"]);
    const menuHomes = new Set(["file-menu", "edit-menu", "view-menu", "help-menu", "native-menu"]);
    const unlisted = desktopRegistry()
      .list()
      .filter((c) => c.home.some((h) => menuHomes.has(h)) && !listed.has(c.id) && !elsewhere.has(c.id))
      .map((c) => c.id);
    expect(unlisted).toEqual([]);
  });

  it("names the registry commands it reports state for once each", () => {
    const ids = menuCommandIds(menus());
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain("view.page-layout");
  });
});
