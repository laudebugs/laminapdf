import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { manifest, precacheList, precacheVersion } from "../plugins/pwa.ts";
import { APP_ICONS } from "../vite.config.ts";

describe("the manifest", () => {
  it("names each app icon, which the website's script draws", () => {
    const icons = (manifest().icons as { src: string }[]).map((icon) => icon.src.slice(1));
    expect([...new Set(icons)].sort()).toEqual([...APP_ICONS].sort());
    for (const icon of APP_ICONS)
      expect(existsSync(new URL(`../../site/public/${icon}`, import.meta.url)), icon).toBe(true);
  });

  it("installs as a standalone app that opens PDFs", () => {
    expect(manifest()).toMatchObject({
      start_url: "/",
      scope: "/",
      display: "standalone",
      file_handlers: [{ action: "/", accept: { "application/pdf": [".pdf"] } }],
    });
  });
});

describe("the precache", () => {
  it("installs the shell and assets, and leaves the unused backends and the server's rules out", () => {
    expect(
      precacheList([
        "index.html",
        "assets/index-a1.js",
        "assets/lamina_pdf_extract-b2.wasm",
        "assets/lamina_pdf_text-c3.wasm",
        "assets/lamina_pdf_render_hayro-d4.wasm",
        "assets/index-a1.js.map",
        "appearance.js",
        "_headers",
        "sw.js",
      ]),
    ).toEqual(["/", "/appearance.js", "/assets/index-a1.js", "/assets/lamina_pdf_extract-b2.wasm"]);
  });

  it("changes version when any file's contents change, whatever the order", () => {
    const a = new Map([
      ["/", new Uint8Array([1])],
      ["/x.js", new Uint8Array([2])],
    ]);
    const reordered = new Map([...a].reverse());
    const changed = new Map([...a, ["/x.js", new Uint8Array([3])]]);
    expect(precacheVersion(a)).toBe(precacheVersion(reordered));
    expect(precacheVersion(a)).not.toBe(precacheVersion(changed));
  });
});
