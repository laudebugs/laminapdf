// Every component `@laminapdf/ui` exports appears on a gallery route (todo U6f), and the image the
// tests run in is the one that matches the installed Playwright.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { EXAMPLES } from "@laminapdf/ui/testing";
import { describe, expect, it } from "vitest";
import { ROUTES } from "../src/routes.ts";

/** Exported components drawn inside another one's examples, with that one. */
const NESTED: Readonly<Record<string, string>> = {
  Radio: "RadioGroup",
  TabPanel: "SegmentedTabs",
  AppearanceFields: "ViewSettingsButton",
  AppearanceSettings: "SettingsDialog",
};

/** Exports that draw nothing themselves, with why. */
const NOT_DRAWN: Readonly<Record<string, string>> = {
  CommandError: "the error a command registry throws, not a component",
  CommandRegistry: "the store of commands that the palette, toolbars and menus read",
  CommandsProvider: "a context provider",
  DensityProvider: "a context provider: the gallery's density control sets it on every route",
  PlatformProvider: "a context provider",
  ModeRegistry: "the shell's store of modes, which the shell-regions route drives",
  ToolRegistry: "the store of interaction modes (tools), which draws nothing",
  ReaderController: "a state controller, which the reader routes drive",
  RecentCommands: "the palette's list of recent commands, kept in storage",
  RegistryPalette: "CommandPalette (on the overlays route) filled from a command registry",
  ShellLayout: "the shell's layout store, which the shell-regions route drives",
  ToastQueue: "the toast region's queue, not a component",
  Announcer: "the live regions' store of what to say, not a component",
  LiveRegions: "two live regions screen readers hear, drawn out of sight",
  SettingsRegistry: "the Settings dialog's store of sections, which the dialogs route fills",
};

/** The package's entries that export components: the root, `./dialogs`, `./editing` and `./reader-mode`. */
const ENTRIES = ["index.ts", "dialogs.ts", "editing.ts", "reader-mode.ts"];

/** The value exports of @laminapdf/ui whose names start with a capital (components and classes). */
function exportedComponents(): string[] {
  const names = ENTRIES.flatMap((entry) => {
    const source = readFileSync(new URL(`../../../packages/ui/src/${entry}`, import.meta.url), "utf8");
    return [...source.matchAll(/export\s*\{([^}]*)\}/g)].flatMap((m) =>
      (m[1] ?? "").split(",").map((part) => part.trim()),
    );
  });
  return names.filter((name) => /^[A-Z][a-z]/.test(name) && !name.startsWith("type "));
}

describe("the gallery's coverage of @laminapdf/ui", () => {
  const onRoutes = new Set(ROUTES.flatMap((r) => [...r.components, ...(r.renders ?? [])]));

  it("shows every exported component on a route", () => {
    const missing = exportedComponents().filter((name) => {
      const parent = NESTED[name];
      return !(onRoutes.has(name) || (parent !== undefined && onRoutes.has(parent)) || name in NOT_DRAWN);
    });
    expect(missing).toEqual([]);
  });

  it("gives every component of the shared state list a route, and every route's components states", () => {
    const withStates = new Set(EXAMPLES.map((e) => e.component));
    const routed = new Set(ROUTES.flatMap((r) => r.components));
    expect([...withStates].filter((c) => !routed.has(c))).toEqual([]);
    expect([...routed].filter((c) => !withStates.has(c))).toEqual([]);
  });

  it("lists only names the package exports", () => {
    const exported = new Set(exportedComponents());
    const listed = [...onRoutes, ...Object.keys(NESTED), ...Object.keys(NOT_DRAWN)];
    expect(listed.filter((name) => !exported.has(name))).toEqual([]);
  });
});

describe("the Playwright image", () => {
  it("matches the installed @playwright/test", () => {
    const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as {
      config: { playwrightImage: string };
    };
    const require = createRequire(import.meta.url);
    const installed = (require("@playwright/test/package.json") as { version: string }).version;
    expect(manifest.config.playwrightImage).toBe(`mcr.microsoft.com/playwright:v${installed}-noble`);
  });
});
