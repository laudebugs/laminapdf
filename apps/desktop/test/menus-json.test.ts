// `src-tauri/menus.json` is the menus' data as `src/menus/menus.ts` makes it: the main process
// embeds the file, so a change to the layout or to a command's title or keys must reach it.
import { readFileSync, writeFileSync } from "node:fs";
import { env } from "node:process";
import { createWebPlatform } from "@laminapdf/ui";
import { expect, it } from "vitest";
import { desktopRegistry, nativeMenus } from "../src/menus/menus.ts";
import { createDesktopPlatform } from "../src/platform/desktop.ts";

const FILE = new URL("../src-tauri/menus.json", import.meta.url);

it("src-tauri/menus.json is the menus' data (pnpm --filter @laminapdf/desktop menus writes it)", () => {
  const menus = nativeMenus(desktopRegistry(), { platform: createDesktopPlatform(createWebPlatform()) });
  const text = `${JSON.stringify(menus, null, 2)}\n`;
  if (env.LAMINA_WRITE_MENUS === "1") writeFileSync(FILE, text);
  expect(
    readFileSync(FILE, "utf8"),
    "src-tauri/menus.json is out of date: run pnpm --filter @laminapdf/desktop menus",
  ).toBe(text);
});
