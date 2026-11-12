// Moving inline styles into a style sheet (src/inline-styles.mjs): the markup it rewrites and leaves,
// the rules it writes, the selectors it translates or refuses, and a small built site end to end.

import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  classFor,
  moveInlineStyles,
  moveInlineStylesIn,
  ruleFor,
  translateStyleSelectors,
} from "../src/inline-styles.mjs";

describe("moveInlineStyles", () => {
  it("gives each style attribute a class, adding to the class an element has", () => {
    const { html, styles } = moveInlineStyles(
      `<span style="--0:#82AAFF;--1:#3B61B0">a</span><li class="item" style="--depth: 1;">b</li>`,
    );
    const token = classFor("--0:#82AAFF;--1:#3B61B0");
    const depth = classFor("--depth: 1;");
    expect(html).toBe(`<span class="${token}">a</span><li class="item ${depth}">b</li>`);
    expect([...styles]).toEqual([
      [token, "--0:#82AAFF;--1:#3B61B0"],
      [depth, "--depth: 1;"],
    ]);
  });

  it("names one class per distinct style, by whether it starts with a custom property", () => {
    expect(classFor("--0:#fff")).toBe(classFor(" --0:#fff "));
    expect(classFor("--0:#fff")).toMatch(/^lamina-v-[0-9a-f]{10}$/);
    expect(classFor("padding:0")).toMatch(/^lamina-s-[0-9a-f]{10}$/);
  });

  it("leaves text in scripts and comments alone", () => {
    const page = `<script>el.innerHTML = '<b style="color:red">';</script><!-- <i style="x:y"> -->`;
    expect(moveInlineStyles(page)).toEqual({ html: page, styles: new Map() });
  });

  it("gives a style a page's script removes the class named for it, and no rule", () => {
    const { html, styles } = moveInlineStyles(`<kbd class="sl-hidden" style="display: none;">K</kbd>`, [
      { tag: "kbd", style: "display: none;", className: "shortcut" },
    ]);
    expect(html).toBe(`<kbd class="sl-hidden shortcut">K</kbd>`);
    expect(styles.size).toBe(0);
  });

  it("refuses an inline <style> element", () => {
    expect(() => moveInlineStyles("<head><style>p{}</style></head>")).toThrow(/inlineStylesheets/);
  });
});

describe("ruleFor", () => {
  it("makes every declaration !important, once", () => {
    expect(ruleFor("c", "padding:0; color: red !important;")).toBe(".c{padding:0 !important;color: red !important}");
  });
});

describe("translateStyleSelectors", () => {
  it("rewrites Expressive Code's token selector to the classes for custom properties", () => {
    expect(translateStyleSelectors(`.ec-line :where(span[style^='--']:not([class])){color:var(--0)}`)).toBe(
      `.ec-line :where(span[class^='lamina-v-']){color:var(--0)}`,
    );
  });

  it("refuses any other selector on the style attribute", () => {
    expect(() => translateStyleSelectors(`div[style*="color"]{x:y}`)).toThrow(/selects by the style attribute/);
  });
});

describe("moveInlineStylesIn", () => {
  let dir: string | undefined;
  afterEach(async () => {
    if (dir !== undefined) await rm(dir, { recursive: true, force: true });
  });

  it("writes one style sheet, links it from the pages that had a style, and translates the site's style sheets", async () => {
    dir = await mkdtemp(join(tmpdir(), "inline-styles-"));
    await mkdir(join(dir, "_astro"));
    await mkdir(join(dir, "plain"));
    await writeFile(join(dir, "index.html"), `<html><head></head><body><span style="--0:#fff">a</span></body></html>`);
    await writeFile(join(dir, "plain", "index.html"), "<html><head></head><body>b</body></html>");
    await writeFile(join(dir, "_astro", "ec.css"), `.ec-line span[style^='--']:not([class]){color:var(--0)}`);
    const url = pathToFileURL(`${dir}/`);
    expect(await moveInlineStylesIn(url)).toEqual({ pages: 1, classes: 1 });
    const sheet = (await readdir(join(dir, "_astro"))).find((name) => name.startsWith("inline-styles."));
    expect(sheet).toMatch(/^inline-styles\.[0-9a-f]{8}\.css$/);
    expect(await readFile(join(dir, "_astro", sheet ?? ""), "utf8")).toBe(
      `.${classFor("--0:#fff")}{--0:#fff !important}\n`,
    );
    expect(await readFile(join(dir, "index.html"), "utf8")).toContain(
      `<link rel="stylesheet" href="/_astro/${sheet}"></head>`,
    );
    expect(await readFile(join(dir, "plain", "index.html"), "utf8")).not.toContain("inline-styles");
    expect(await readFile(join(dir, "_astro", "ec.css"), "utf8")).toBe(
      ".ec-line span[class^='lamina-v-']{color:var(--0)}",
    );
  });
});
