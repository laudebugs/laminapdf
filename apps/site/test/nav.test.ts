// The site header's links: §7.39's order, and a page's link only once the page exists.

import { describe, expect, it } from "vitest";
import { routeOf, siteNav } from "../src/nav.js";

describe("routeOf", () => {
  it("serves a page file at its folder's path", () => {
    expect(routeOf("/src/pages/index.astro")).toBe("/");
    expect(routeOf("/src/pages/privacy.astro")).toBe("/privacy/");
    expect(routeOf("/src/pages/download/index.astro")).toBe("/download/");
    expect(routeOf("/src/pages/desktop/releases/notes.md")).toBe("/desktop/releases/notes/");
  });
});

describe("siteNav", () => {
  it("links only to the docs while the site has none of §7.39's pages", () => {
    expect(siteNav(["/src/pages/index.astro", "/src/pages/404.astro"])).toEqual([
      { label: "Docs", href: "https://docs.laminapdf.com/" },
    ]);
  });

  it("adds each page's link once the page exists, in §7.39's order", () => {
    const pages = ["/src/pages/privacy.astro", "/src/pages/download/index.astro", "/src/pages/index.astro"];
    expect(siteNav(pages).map((link) => `${link.label} ${link.href}`)).toEqual([
      "Download /download/",
      "Docs https://docs.laminapdf.com/",
      "Privacy /privacy/",
    ]);
  });
});
