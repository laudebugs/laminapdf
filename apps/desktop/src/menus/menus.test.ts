import { type CommandContext, createWebPlatform } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { createDesktopPlatform } from "../platform/desktop.ts";
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

  it("names the registry commands it reports state for once each", () => {
    const ids = menuCommandIds(menus());
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toContain("view.page-layout");
  });
});
