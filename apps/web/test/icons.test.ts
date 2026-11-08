import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { WEBSITE_ICONS } from "../vite.config";

// The reader shows the website's icon (vite.config.ts's `websiteIcons` copies the files): every
// icon is linked by index.html and exists in the website's public/.
describe("the website's icons", () => {
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const linked = [...html.matchAll(/<link rel="(?:icon|apple-touch-icon)" href="\/([^"]+)"/g)].map(([, file]) => file);

  it("are each linked once by index.html", () => {
    expect([...linked].sort()).toEqual([...WEBSITE_ICONS].sort());
  });

  it("exist in the website's public folder", () => {
    for (const icon of WEBSITE_ICONS) {
      expect(existsSync(new URL(`../../site/public/${icon}`, import.meta.url)), icon).toBe(true);
    }
  });
});
