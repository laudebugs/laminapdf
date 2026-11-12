// Inline `style` attributes as classes, after the build, so a site's CSP needs no 'unsafe-inline'
// in style-src. Starlight and Expressive Code write style attributes (a sidebar list's depth, an
// icon's size, every code token's colours), and a CSP without 'unsafe-inline' blocks each one. This
// step gives every element with a style attribute a class named after the attribute's value, writes
// one style sheet with a rule per value, and links it first in every page's head. Node-only: it runs
// at build time, never in a page.
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { filesIn } from "./integrations.mjs";

/** The prefix of the classes this step adds, one per distinct style attribute. */
const PREFIX = "s-";

/**
 * The class added beside a style class when the style sets only custom properties (`--0:#fff`) on
 * an element with no class of its own. Expressive Code colours its tokens with such spans and finds
 * them with `span[style^='--']:not([class])`; {@link rewriteSelectors} points that selector here.
 */
export const VARS_CLASS = "s-vars";

/** The selector Expressive Code's style sheet finds its token spans with, and its replacement. */
const TOKEN_SELECTOR = "span[style^='--']:not([class])";

/**
 * The elements whose contents are text, not markup: a style attribute inside them is text (a
 * script's string, a code sample), so the scan skips from their start tag to their end tag.
 */
const RAW_TEXT = /^(script|style|textarea|title)$/i;

/** A start tag, a comment, or the end of a raw-text element; the scan's tokens. */
const TOKEN =
  /<!--[\s\S]*?-->|<([a-zA-Z][a-zA-Z0-9-]*)((?:\s+[^\s"'>/=]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'=<>`]+))?)*)\s*(\/?)>/g;

/** One attribute in a start tag's attribute text: its name and its value, in any of three quotings. */
const ATTRIBUTE = /(\s+)([^\s"'>/=]+)(?:(\s*=\s*)(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;

/**
 * The class for a style attribute's value: the prefix and the first 10 hex digits of its SHA-256,
 * so the same value gets the same class on every page and in every build.
 *
 * @param {string} style the attribute's value, entities decoded
 * @returns {string}
 */
export function styleClass(style) {
  return PREFIX + createHash("sha256").update(style).digest("hex").slice(0, 10);
}

/**
 * The value of an HTML attribute with the entities a build writes decoded: `&quot;`, `&#39;`,
 * `&lt;`, `&gt;`, then `&amp;`.
 *
 * @param {string} value
 * @returns {string}
 */
function decoded(value) {
  return value
    .replaceAll("&quot;", '"')
    .replaceAll("&#39;", "'")
    .replaceAll("&#x27;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}

/**
 * `attributes` (a start tag's attribute text) with its style attribute replaced by classes: the
 * style's class, and {@link VARS_CLASS} when the style sets only custom properties and the element
 * had no class. `styles` collects every style seen. Returns the text unchanged when it has no style.
 *
 * @param {string} attributes
 * @param {Map<string, string>} styles class → style, filled in
 * @returns {string}
 */
function withoutStyle(attributes, styles) {
  /** @type {{ name: string, text: string, value: string | undefined }[]} */
  const parsed = [];
  for (const match of attributes.matchAll(ATTRIBUTE)) {
    const [text, , name = "", , double, single, bare] = match;
    parsed.push({ name: name.toLowerCase(), text, value: double ?? single ?? bare });
  }
  const style = parsed.find((attribute) => attribute.name === "style");
  if (style === undefined) return attributes;
  const value = decoded(style.value ?? "").trim();
  const kept = parsed.filter((attribute) => attribute !== style);
  if (value === "") return kept.map((attribute) => attribute.text).join("");
  const name = styleClass(value);
  styles.set(name, value);
  const existing = kept.find((attribute) => attribute.name === "class");
  const onlyVariables = value.split(";").every((part) => part.trim() === "" || part.trim().startsWith("--"));
  const added = existing === undefined && onlyVariables ? [name, VARS_CLASS] : [name];
  if (existing === undefined) return `${kept.map((a) => a.text).join("")} class="${added.join(" ")}"`;
  const classes = `${(existing.value ?? "").trim()} ${added.join(" ")}`.trim();
  return kept.map((a) => (a === existing ? ` class="${classes}"` : a.text)).join("");
}

/**
 * `html` with every style attribute replaced by classes (see {@link withoutStyle}), skipping
 * comments and the text of raw-text elements; `styles` collects the styles seen.
 *
 * @param {string} html
 * @param {Map<string, string>} styles class → style, filled in
 * @returns {string}
 */
export function classesForStyles(html, styles) {
  let out = "";
  let from = 0;
  TOKEN.lastIndex = 0;
  for (let match = TOKEN.exec(html); match !== null; match = TOKEN.exec(html)) {
    const [whole, name, attributes = "", selfClosing = ""] = match;
    out += html.slice(from, match.index);
    from = match.index + whole.length;
    if (name === undefined) {
      out += whole;
      continue;
    }
    out += `<${name}${withoutStyle(attributes, styles)}${selfClosing === "" ? "" : " /"}>`;
    if (RAW_TEXT.test(name)) {
      const end = html.toLowerCase().indexOf(`</${name.toLowerCase()}`, from);
      const stop = end === -1 ? html.length : end;
      out += html.slice(from, stop);
      from = stop;
      TOKEN.lastIndex = stop;
    }
  }
  return out + html.slice(from);
}

/** The cascade layer the style sheet's rules sit in; declared first, so it ranks below every other. */
export const LAYER = "lamina-inline-styles";

/**
 * Whether `html` has a `<style>` element outside its scripts and comments.
 *
 * @param {string} html
 * @returns {boolean}
 */
function hasStyleElement(html) {
  const markup = html.replace(/<!--[\s\S]*?-->/g, "").replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "");
  return /<style[\s>]/i.test(markup);
}

/**
 * The style sheet for `styles`: a rule per class, sorted, so the file is the same for the same
 * pages, in a cascade layer of its own. A style attribute beats every rule; these rules lose to
 * every other, because the sheet is linked first and its layer is declared before Starlight's.
 * That is what the pages need: Starlight's search writes `display: none` on its shortcut hint and
 * clears it from a script once the page runs (`shortcut.style.display = ''`), which can't clear a
 * class, so Starlight's own `md:sl-flex` has to win, as it does once that script has run. The other
 * styles set custom properties or values no other rule sets, and the tests compare pictures of
 * the pages before and after.
 *
 * @param {Map<string, string>} styles class → style
 * @returns {string}
 */
export function styleSheet(styles) {
  const rules = [...styles]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([name, style]) => `  .${name}{${style.replace(/;\s*$/, "")}}`);
  return `@layer ${LAYER} {\n${rules.join("\n")}\n}\n`;
}

/**
 * `css` with the selector that finds elements by a custom-properties style attribute pointed at
 * {@link VARS_CLASS}, and whether it was there.
 *
 * @param {string} css
 * @returns {{ css: string, rewritten: boolean }}
 */
export function rewriteSelectors(css) {
  const variants = [TOKEN_SELECTOR, TOKEN_SELECTOR.replaceAll("'", '"'), TOKEN_SELECTOR.replace(/'/g, "")];
  let rewritten = false;
  let out = css;
  for (const selector of variants) {
    if (!out.includes(selector)) continue;
    out = out.replaceAll(selector, `span.${VARS_CLASS}`);
    rewritten = true;
  }
  return { css: out, rewritten };
}

/**
 * The pages under `dir` with their style attributes replaced by classes, keyed by file, for the
 * pages that had any; `styles` collects every style seen. Fails on a `<style>` element.
 *
 * @param {URL} dir
 * @param {Map<string, string>} styles class → style, filled in
 * @returns {Promise<Map<URL, string>>}
 */
async function convertedPages(dir, styles) {
  /** @type {Map<URL, string>} */
  const pages = new Map();
  for (const file of await filesIn(dir, ".html")) {
    const html = await readFile(file, "utf8");
    if (hasStyleElement(html)) {
      throw new Error(
        `${file.pathname}: has a <style> element, which style-src 'self' blocks: set build.inlineStylesheets to "never"`,
      );
    }
    const seen = new Map();
    const out = classesForStyles(html, seen);
    for (const [name, style] of seen) styles.set(name, style);
    if (seen.size > 0) pages.set(file, out);
  }
  return pages;
}

/** A page's head start tag, and its charset, which must stay within the first 1,024 bytes. */
const HEAD = /<head(?:\s[^>]*)?>(?:\s*<meta charset[^>]*>)?/i;

/**
 * Points Expressive Code's token selector at {@link VARS_CLASS} in every style sheet under `dir`,
 * and fails when none held it.
 *
 * @param {URL} dir
 * @returns {Promise<void>}
 */
async function rewriteStyleSheets(dir) {
  let rewritten = false;
  for (const file of await filesIn(dir, ".css")) {
    const result = rewriteSelectors(await readFile(file, "utf8"));
    if (!result.rewritten) continue;
    rewritten = true;
    await writeFile(file, result.css);
  }
  if (!rewritten) {
    throw new Error(
      `pages have custom-properties styles, but no style sheet holds ${TOKEN_SELECTOR} to point at .${VARS_CLASS}: ` +
        "Expressive Code may have changed how it finds its token spans",
    );
  }
}

/**
 * Replaces every style attribute in the build in `dir` with classes, writes their style sheet to
 * `_astro/inline-styles.<hash>.css`, links it first in every page's `<head>` that had a style, and
 * points Expressive Code's token selector at {@link VARS_CLASS}. Fails on a `<style>` element, and
 * when a page gets a custom-properties class but no style sheet holds the selector to rewrite (a
 * changed Expressive Code), since the code would then lose its colours.
 *
 * @param {URL} dir the build's output folder
 * @returns {Promise<{ pages: number, styles: number }>}
 */
export async function moveInlineStyles(dir) {
  /** @type {Map<string, string>} */
  const styles = new Map();
  const pages = await convertedPages(dir, styles);
  if (pages.size === 0) return { pages: 0, styles: 0 };
  const css = styleSheet(styles);
  const name = `_astro/inline-styles.${createHash("sha256").update(css).digest("hex").slice(0, 8)}.css`;
  await mkdir(new URL("_astro/", dir), { recursive: true });
  await writeFile(new URL(name, dir), css);
  const link = `<link rel="stylesheet" href="/${name}">`;
  for (const [file, html] of pages) {
    if (!HEAD.test(html)) throw new Error(`${file.pathname}: has a style attribute but no <head>`);
    await writeFile(
      file,
      html.replace(HEAD, (start) => start + link),
    );
  }
  if ([...pages.values()].some((html) => html.includes(VARS_CLASS))) await rewriteStyleSheets(dir);
  return { pages: pages.size, styles: styles.size };
}

/**
 * The build step: {@link moveInlineStyles} on the build's output. List it before
 * `inlineScriptHashes`, so the scripts it hashes are the ones the pages ship.
 *
 * @type {import("astro").AstroIntegration}
 */
export const inlineStylesAsClasses = {
  name: "lamina-inline-styles-as-classes",
  hooks: { "astro:build:done": ({ dir }) => moveInlineStyles(dir).then(() => undefined) },
};
