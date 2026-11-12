// Moving inline styles into a style sheet (src/inline-styles.mjs): the markup it rewrites and leaves,
// the rules it writes, the selectors it translates or refuses, and a small built site end to end.

import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  classFor,
  hasInlineStyle,
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

  it("leaves another attribute's value alone, though it holds markup", () => {
    // Expressive Code's copy button keeps the raw code in data-code, `<` and `'` unescaped.
    const page = `<button data-code="a <span style='color:red'> b" class="copy">c</button>`;
    expect(moveInlineStyles(page)).toEqual({ html: page, styles: new Map() });
  });

  it("keeps a single-quoted class whole once it is written in double quotes", () => {
    const { html } = moveInlineStyles(`<p class='a "b"' style="x:1">c</p>`);
    expect(html).toBe(`<p class="a &quot;b&quot; ${classFor("x:1")}">c</p>`);
  });

  it("keeps a self-closing tag self-closed", () => {
    const { html } = moveInlineStyles(`<svg><rect style="fill:red"/><circle r="1"/></svg>`);
    expect(html).toBe(`<svg><rect class="${classFor("fill:red")}" /><circle r="1"/></svg>`);
  });

  it("reads a value's character references, and a style in single or no quotes", () => {
    const { styles } = moveInlineStyles(
      `<i style="font-family: &quot;A&quot;">a</i><b style='x:1'>b</b><u style=y:2>c</u>`,
    );
    expect([...styles.values()]).toEqual(['font-family: "A"', "x:1", "y:2"]);
  });

  it("refuses an inline <style> element", () => {
    expect(() => moveInlineStyles("<head><style>p{}</style></head>")).toThrow(/inlineStylesheets/);
  });
});

describe("ruleFor", () => {
  it("makes every declaration !important, once", () => {
    expect(ruleFor("c", "padding:0; color: red !important;")).toBe(".c{padding:0 !important;color: red !important}");
  });

  it("splits declarations only at semicolons outside quotes and parentheses", () => {
    expect(
      ruleFor("c", `font-family: "A;B"; background: url(data:image/png;base64,AA); width: calc(var(--a, 1px))`),
    ).toBe(
      `.c{font-family: "A;B" !important;background: url(data:image/png;base64,AA) !important;width: calc(var(--a, 1px)) !important}`,
    );
  });
});

describe("hasInlineStyle", () => {
  it("finds a style attribute in any quoting and a <style> element, not text that looks like one", () => {
    expect(hasInlineStyle("<p style=x>a</p>")).toBe(true);
    expect(hasInlineStyle("<style>p{}</style>")).toBe(true);
    expect(hasInlineStyle(`<p data-x="<b style='y'>">a</p><script>"<i style=z>"</script>`)).toBe(false);
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

  it("writes one style sheet, links it from the pages that had a style, and renames a translated style sheet", async () => {
    dir = await mkdtemp(join(tmpdir(), "inline-styles-"));
    await mkdir(join(dir, "_astro"));
    await mkdir(join(dir, "plain"));
    // The address in each quoting a page might use.
    const head = `<head><link rel="stylesheet" href="/_astro/ec.css"><link rel=preload href=/_astro/ec.css><link href='/_astro/ec.css'></head>`;
    await writeFile(join(dir, "index.html"), `<html>${head}<body><span style="--0:#fff">a</span></body></html>`);
    await writeFile(join(dir, "plain", "index.html"), `<html>${head}<body>b</body></html>`);
    await writeFile(join(dir, "_astro", "ec.css"), `.ec-line span[style^='--']:not([class]){color:var(--0)}`);
    expect(await moveInlineStylesIn(pathToFileURL(`${dir}/`))).toEqual({ pages: 1, classes: 1 });
    const files = await readdir(join(dir, "_astro"));
    const sheet = files.find((name) => name.startsWith("inline-styles."));
    expect(sheet).toMatch(/^inline-styles\.[0-9a-f]{8}\.css$/);
    expect(await readFile(join(dir, "_astro", sheet ?? ""), "utf8")).toBe(
      `.${classFor("--0:#fff")}{--0:#fff !important}\n`,
    );
    // A reader's browser keeps the old ec.css for a year: the translated one has a new name.
    const ec = files.find((name) => /^ec\.[0-9a-f]{8}\.css$/.test(name));
    expect(files).not.toContain("ec.css");
    expect(await readFile(join(dir, "_astro", ec ?? ""), "utf8")).toBe(
      ".ec-line span[class^='lamina-v-']{color:var(--0)}",
    );
    const page = await readFile(join(dir, "index.html"), "utf8");
    expect([
      page.includes(`href="/_astro/${ec}"`),
      page.includes(`href=/_astro/${ec}>`),
      page.includes(`href='/_astro/${ec}'`),
    ]).toEqual([true, true, true]);
    expect(page).not.toContain("/_astro/ec.css");
    expect(page).toContain(`<link rel="stylesheet" href="/_astro/${sheet}"></head>`);
    const plain = await readFile(join(dir, "plain", "index.html"), "utf8");
    expect(plain).toContain(`href="/_astro/${ec}"`);
    expect(plain).not.toContain("inline-styles");
  });

  it("writes a page whose only style a script removes", async () => {
    dir = await mkdtemp(join(tmpdir(), "inline-styles-"));
    await mkdir(join(dir, "_astro"));
    await writeFile(
      join(dir, "index.html"),
      `<html><head></head><body><kbd style="display: none;">K</kbd></body></html>`,
    );
    const scripted = [{ tag: "kbd", style: "display: none;", className: "shortcut" }];
    expect(await moveInlineStylesIn(pathToFileURL(`${dir}/`), scripted)).toEqual({ pages: 0, classes: 0 });
    expect(await readFile(join(dir, "index.html"), "utf8")).toContain(`<kbd class="shortcut">K</kbd>`);
  });
});
