// The diagrams' rules (docs/guidelines/docs-writing.md, "Diagrams"), checked without a build:
// colours come only from the --paper-* tokens, through classes; ids come only from a drawing's
// prefix; every diagram has a caption, a text description and a drawing; every page names a
// diagram that exists. The browser test (test/browser/) checks how they look.
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { test } from "node:test";
import { DIAGRAMS, drawingFile } from "../src/components/diagrams/registry.mjs";

const ROOT = new URL("../", import.meta.url);
const DRAWINGS = new URL("src/components/diagrams/", ROOT);
const KIT = new URL("kit/", DRAWINGS);
const STYLES = new URL("src/styles/diagrams.css", ROOT);
const PAGES = new URL("src/content/docs/", ROOT);

/** The `.astro` files in `dir`, as [name, source]. */
function astroFiles(/** @type {URL} */ dir) {
  return readdirSync(dir)
    .filter((f) => f.endsWith(".astro"))
    .map((f) => /** @type {[string, string]} */ ([f, readFileSync(new URL(f, dir), "utf8")]));
}

/** Every `.mdx` page under the docs, as [path, source]. */
function pages(/** @type {URL} */ dir = PAGES, /** @type {string} */ prefix = "") {
  /** @type {[string, string][]} */
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) out.push(...pages(new URL(`${entry.name}/`, dir), `${prefix}${entry.name}/`));
    else if (entry.name.endsWith(".mdx"))
      out.push([`${prefix}${entry.name}`, readFileSync(new URL(entry.name, dir), "utf8")]);
  }
  return out;
}

const COLOUR = /#[0-9a-f]{3,8}\b|\brgba?\(|\bhsla?\(/i;
const files = [...astroFiles(DRAWINGS), ...astroFiles(KIT)];

test("no diagram file writes a colour, a style attribute or a colour attribute", () => {
  for (const [name, source] of files) {
    assert.doesNotMatch(source, COLOUR, `${name} writes a colour; use a class from styles/diagrams.css`);
    assert.doesNotMatch(source, /\sstyle=/, `${name} has a style attribute; the website's CSP forbids them`);
    assert.doesNotMatch(source, /\s(fill|stroke|color)="/, `${name} sets a colour attribute; use a class`);
  }
});

test("every colour in the diagrams' stylesheet is a token", () => {
  const css = readFileSync(STYLES, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const values = [...css.matchAll(/(?:^|[;{\s])(fill|stroke|color)\s*:\s*([^;]+);/g)];
  assert.ok(values.length > 0, "no colours found: did the stylesheet move?");
  for (const [, property, value] of values) {
    assert.match(
      value?.trim() ?? "",
      /^(var\(--paper-[a-z0-9-]+\)|none|currentColor)$/,
      `${property}: ${value} in diagrams.css is not a --paper-* token`,
    );
  }
});

test("drawings take their ids from their prefix, so two on one page never clash", () => {
  for (const [name, source] of astroFiles(DRAWINGS)) {
    assert.doesNotMatch(source, /\sid="/, `${name} writes a fixed id; build it from the prefix prop`);
  }
});

test("every diagram has a caption, a text description and a drawing", () => {
  const drawings = new Set(astroFiles(DRAWINGS).map(([f]) => f));
  for (const [name, { caption, text }] of Object.entries(DIAGRAMS)) {
    assert.ok(caption.trim().length > 0 && caption.length <= 120, `${name}: a caption of 1 to 120 characters`);
    assert.ok(
      text.trim().length >= caption.length,
      `${name}: the text description says at least what the caption says`,
    );
    assert.ok(drawings.has(drawingFile(name)), `${name}: no drawing ${drawingFile(name)}`);
  }
  for (const file of drawings) {
    const name = Object.keys(DIAGRAMS).find((n) => drawingFile(n) === file);
    assert.ok(name, `${file} has no entry in registry.mjs`);
  }
});

test("every page names a diagram that exists, and never before its opening paragraph", () => {
  for (const [path, source] of pages()) {
    const body = source
      .replace(/^---[\s\S]*?---\s*/, "")
      .replace(/^import .*$/gm, "")
      .trim();
    for (const [, name] of source.matchAll(/<Diagram\s+name="([^"]+)"/g)) {
      assert.ok(name !== undefined && name in DIAGRAMS, `${path}: no diagram named "${name}"`);
    }
    assert.ok(!body.startsWith("<Diagram"), `${path}: a diagram comes before the opening paragraph`);
  }
});
