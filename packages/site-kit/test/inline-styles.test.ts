// Style attributes as classes: each test builds a small dist/ folder, runs the step, and reads back
// the pages and the style sheet it wrote.

import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  classesForStyles,
  LAYER,
  moveInlineStyles,
  rewriteSelectors,
  styleClass,
  styleSheet,
  VARS_CLASS,
} from "../src/inline-styles.mjs";

const folders: string[] = [];

afterEach(async () => {
  for (const folder of folders.splice(0)) await rm(folder, { recursive: true, force: true });
});

/** A dist/ folder holding `files` (path → text), as a URL ending in a slash. */
async function dist(files: Record<string, string>): Promise<URL> {
  const root = await mkdtemp(join(tmpdir(), "inline-styles-"));
  folders.push(root);
  for (const [path, text] of Object.entries(files)) {
    await mkdir(join(root, path, ".."), { recursive: true });
    await writeFile(join(root, path), text);
  }
  return pathToFileURL(`${root}/`);
}

/** `html` through {@link classesForStyles}, with the styles it found. */
function converted(html: string): { html: string; styles: Map<string, string> } {
  const styles = new Map<string, string>();
  return { html: classesForStyles(html, styles), styles };
}

describe("classesForStyles", () => {
  it("should replace a style attribute with a class named after its value", () => {
    const { html, styles } = converted('<dialog style="padding:0" aria-label="Search">');
    const name = styleClass("padding:0");
    expect(html).toBe(`<dialog aria-label="Search" class="${name}">`);
    expect([...styles]).toEqual([[name, "padding:0"]]);
  });

  it("should add the class to an element's own classes", () => {
    const { html } = converted('<svg class="icon" style="--sl-icon-size: 1em;" width="16">');
    expect(html).toBe(`<svg class="icon ${styleClass("--sl-icon-size: 1em;")}" width="16">`);
  });

  it("should mark a classless element that sets only custom properties", () => {
    const { html } = converted('<span style="--0:#D6DEEB;--1:#403F53">const</span>');
    expect(html).toBe(`<span class="${styleClass("--0:#D6DEEB;--1:#403F53")} ${VARS_CLASS}">const</span>`);
  });

  it("should give the same value the same class on every element", () => {
    const { html, styles } = converted('<li style="--depth: 0;"><ul style="--depth: 0;">');
    expect(styles.size).toBe(1);
    expect(html.match(new RegExp(styleClass("--depth: 0;"), "g"))).toHaveLength(2);
  });

  it("should decode entities in a value before naming it", () => {
    const { styles } = converted('<p style="font-family:&quot;A B&quot;">');
    expect([...styles.values()]).toEqual(['font-family:"A B"']);
  });

  it("should leave style text inside scripts and comments alone", () => {
    const html = '<script>el.innerHTML = \'<b style="x">\';</script><!-- <p style="y"> --><p>';
    expect(converted(html)).toEqual({ html, styles: new Map() });
  });

  it("should drop an empty style attribute", () => {
    expect(converted('<p style="">').html).toBe("<p>");
  });
});

describe("styleSheet", () => {
  it("should put one rule per class, sorted, in its own layer", () => {
    const css = styleSheet(
      new Map([
        ["s-b", "padding:0;"],
        ["s-a", "--depth: 0"],
      ]),
    );
    expect(css).toBe(`@layer ${LAYER} {\n  .s-a{--depth: 0}\n  .s-b{padding:0}\n}\n`);
  });
});

describe("rewriteSelectors", () => {
  it("should point Expressive Code's token selector at the marker class", () => {
    const css = ".ec-line :where(span[style^='--']:not([class])){color:var(--0)}";
    expect(rewriteSelectors(css)).toEqual({
      css: `.ec-line :where(span.${VARS_CLASS}){color:var(--0)}`,
      rewritten: true,
    });
  });

  it("should leave other style sheets as they are", () => {
    expect(rewriteSelectors("p{color:red}")).toEqual({ css: "p{color:red}", rewritten: false });
  });
});

describe("moveInlineStyles", () => {
  it("should write the style sheet and link it after the charset of every page with a style", async () => {
    const dir = await dist({
      "index.html":
        '<!doctype html><html><head><meta charset="utf-8"><title>T</title></head><body><p style="padding:0">',
      "plain/index.html": "<!doctype html><html><head><title>P</title></head><body><p>",
    });
    expect(await moveInlineStyles(dir)).toEqual({ pages: 1, styles: 1 });
    const [sheet] = (await readdir(new URL("_astro/", dir))).filter((f) => f.startsWith("inline-styles."));
    expect(await readFile(new URL(`_astro/${sheet}`, dir), "utf8")).toContain(`.${styleClass("padding:0")}{padding:0}`);
    const index = await readFile(new URL("index.html", dir), "utf8");
    expect(index).toContain(`<meta charset="utf-8"><link rel="stylesheet" href="/_astro/${sheet}"><title>`);
    expect(index).not.toContain("style=");
    expect(await readFile(new URL("plain/index.html", dir), "utf8")).not.toContain("inline-styles");
  });

  it("should fail on a <style> element", async () => {
    const dir = await dist({ "index.html": "<html><head><style>p{color:red}</style></head><body><p>" });
    await expect(moveInlineStyles(dir)).rejects.toThrow(/has a <style> element/);
  });

  it("should fail when custom-properties spans have no selector to find them", async () => {
    const dir = await dist({
      "index.html": '<html><head></head><body><span style="--0:#fff">x</span>',
      "_astro/ec.css": ".ec-line span{color:red}",
    });
    await expect(moveInlineStyles(dir)).rejects.toThrow(/Expressive Code may have changed/);
  });

  it("should rewrite Expressive Code's selector in the build's style sheets", async () => {
    const dir = await dist({
      "index.html": '<html><head></head><body><span style="--0:#fff">x</span>',
      "_astro/ec.css": ".ec-line :where(span[style^='--']:not([class])){color:var(--0)}",
    });
    await moveInlineStyles(dir);
    expect(await readFile(new URL("_astro/ec.css", dir), "utf8")).toContain(`span.${VARS_CLASS}`);
  });
});
