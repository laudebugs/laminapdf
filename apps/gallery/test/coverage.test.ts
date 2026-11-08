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
};

/** Exports that draw nothing themselves, with why. */
const NOT_DRAWN: Readonly<Record<string, string>> = {
  DensityProvider: "a context provider: the gallery's density control sets it on every route",
  PlatformProvider: "a context provider",
  ReaderController: "a state controller, which the reader routes drive",
  ToastQueue: "the toast region's queue, not a component",
};

/** The value exports of @laminapdf/ui whose names start with a capital (components and classes). */
function exportedComponents(): string[] {
  const index = readFileSync(new URL("../../../packages/ui/src/index.ts", import.meta.url), "utf8");
  const names = [...index.matchAll(/export\s*\{([^}]*)\}/g)].flatMap((m) =>
    (m[1] ?? "").split(",").map((part) => part.trim()),
  );
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
