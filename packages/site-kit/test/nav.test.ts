// The header's one list of links on both sites: §7.39's order, a website page's link only once the
// page exists, full addresses across sites, and the current page or section.

import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { pageFiles } from "../src/integrations.mjs";
import { routeOf, siteNav } from "../src/nav.mjs";

describe("routeOf", () => {
  it("serves a page file at its folder's path", () => {
    expect(routeOf("/src/pages/index.astro")).toBe("/");
    expect(routeOf("/src/pages/privacy.astro")).toBe("/privacy/");
    expect(routeOf("/src/pages/download/index.astro")).toBe("/download/");
    expect(routeOf("/src/pages/desktop/releases/notes.md")).toBe("/desktop/releases/notes/");
  });
});

describe("siteNav", () => {
  const pages = ["/src/pages/index.astro", "/src/pages/privacy.astro", "/src/pages/download/index.astro"];

  it("links only to the docs while the website has none of §7.39's pages", () => {
    expect(siteNav(["/src/pages/index.astro"], { site: "website", path: "/" })).toEqual([
      { label: "Docs", href: "https://docs.laminapdf.com/", current: undefined },
    ]);
  });

  it("adds each website page's link once the page exists, in §7.39's order, and marks the current page", () => {
    expect(siteNav(pages, { site: "website", path: "/privacy" })).toEqual([
      { label: "Download", href: "/download/", current: undefined },
      { label: "Docs", href: "https://docs.laminapdf.com/", current: undefined },
      { label: "Privacy", href: "/privacy/", current: "page" },
    ]);
  });

  it("gives the docs site full addresses for the website's pages, and marks Docs on every docs page", () => {
    expect(siteNav(pages, { site: "docs", path: "/guides/save-a-pdf/" })).toEqual([
      { label: "Download", href: "https://laminapdf.com/download/", current: undefined },
      { label: "Docs", href: "/", current: "true" },
      { label: "Privacy", href: "https://laminapdf.com/privacy/", current: undefined },
    ]);
  });
});

describe("pageFiles", () => {
  it("lists a pages folder's .astro and .md files as /src/pages paths, endpoints left out", async () => {
    const root = await mkdtemp(join(tmpdir(), "site-kit-pages-"));
    try {
      for (const file of ["index.astro", "privacy.md", "download/index.astro", "robots.txt.ts"]) {
        await mkdir(join(root, file, ".."), { recursive: true });
        await writeFile(join(root, file), "");
      }
      expect(pageFiles(pathToFileURL(`${root}/`))).toEqual([
        "/src/pages/download/index.astro",
        "/src/pages/index.astro",
        "/src/pages/privacy.md",
      ]);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});
