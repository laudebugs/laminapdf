// Inline styles moved into a style sheet after the build, so a site's CSP needs no 'unsafe-inline'
// in style-src (ADR 0011 §4). Expressive Code colours each code token with a style attribute, and
// Starlight writes a few of its own (the sidebar's depth, an icon's size, the search dialog's
// padding). Each distinct value becomes one class in a style sheet every page with one links. Its
// declarations are !important, so they still win over the site's style sheets, as the attribute did.
// A style sheet that selects elements by their style attribute stops matching once it is gone, so
// the built style sheets' one such selector, Expressive Code's for a token's colour, is rewritten
// to the class, and any other fails the build. Node-only: it runs at build time, never in a page.
import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import { filesIn } from "./integrations.mjs";

/**
 * An inline style a page's own script removes later. A class rule can't be removed that way, so
 * the element gets `className` instead, which the site's CSS styles for as long as the script
 * would have kept the style.
 *
 * @typedef {{ tag: string, style: string, className: string }} ScriptedStyle
 */

/** Comments and scripts, which the rewrite leaves as they are: their text isn't markup. */
const VERBATIM = /(<!--[\s\S]*?-->|<script\b[\s\S]*?<\/script\s*>)/gi;

/** A start tag with a style attribute in double or single quotes: name, the rest, the value. */
const STYLED = /<([a-z][a-z0-9-]*)\b([^>]*?)\sstyle\s*=\s*(?:"([^"]*)"|'([^']*)')([^>]*)>/gi;

/** An inline `<style>` element: Astro writes these unless `build.inlineStylesheets` is "never". */
const STYLE_ELEMENT = /<style[\s>]/i;

/** The prefix of a class for a style that starts with a custom property, and of any other. */
const VARIABLES = "lamina-v-";
const OTHER = "lamina-s-";

/**
 * The class for one inline style's text.
 *
 * @param {string} style
 * @returns {string}
 */
export function classFor(style) {
  const kind = style.trim().startsWith("--") ? VARIABLES : OTHER;
  return `${kind}${createHash("sha256").update(style.trim()).digest("hex").slice(0, 10)}`;
}

/**
 * Selectors on the style attribute, and what each becomes. Expressive Code colours a token with
 * `span[style^='--']:not([class])`: a span whose style starts with a custom property and which has
 * no class. Such a span's class is now exactly one class, starting with {@link VARIABLES}.
 */
/** @type {readonly (readonly [RegExp, string])[]} */
const STYLE_SELECTORS = [[/\[style\^=(['"])--\1\]:not\(\[class\]\)/g, `[class^='${VARIABLES}']`]];

/**
 * `css` with its selectors on the style attribute rewritten to the classes that replace it.
 * Throws on one it can't translate, which would match nothing once the attributes are gone.
 *
 * @param {string} css
 * @returns {string}
 */
export function translateStyleSelectors(css) {
  let out = css;
  for (const [pattern, replacement] of STYLE_SELECTORS) out = out.replace(pattern, replacement);
  const left = /[^{}]*\[style\b[^{}]*\{/.exec(out);
  if (left !== null)
    throw new Error(`selects by the style attribute, which no element keeps: ${left[0].trim().slice(0, 120)}`);
  return out;
}

/**
 * The rule that gives `className` what `style` declares, each declaration !important.
 *
 * @param {string} className
 * @param {string} style
 * @returns {string}
 */
export function ruleFor(className, style) {
  const declarations = style
    .split(";")
    .map((declaration) => declaration.trim().replace(/\s*!important$/i, ""))
    .filter((declaration) => declaration !== "")
    .map((declaration) => `${declaration} !important`);
  return `.${className}{${declarations.join(";")}}`;
}

/**
 * The attributes of a start tag (without its name) with `className` added to its class.
 *
 * @param {string} attributes
 * @param {string} className
 * @returns {string}
 */
function withClass(attributes, className) {
  const existing = /(\sclass\s*=\s*)(?:"([^"]*)"|'([^']*)')/i.exec(attributes);
  if (existing === null) return `${attributes} class="${className}"`;
  const [whole, prefix = "", double, single] = existing;
  const value = (double ?? single ?? "").trim();
  const quote = double === undefined ? "'" : '"';
  const merged = `${prefix}${quote}${value === "" ? className : `${value} ${className}`}${quote}`;
  return attributes.replace(whole, merged);
}

/**
 * `html` with each style attribute replaced by a class, and the styles those classes stand for.
 * Text in comments and scripts is left alone. Throws on an inline `<style>` element, which
 * the CSP would block too.
 *
 * @param {string} html
 * @param {readonly ScriptedStyle[]} [scripted]
 * @returns {{ html: string, styles: Map<string, string> }}
 */
export function moveInlineStyles(html, scripted = []) {
  /** @type {Map<string, string>} */
  const styles = new Map();
  const parts = html.split(VERBATIM).map((part, index) => {
    // split() puts the captured comments and scripts at the odd indices.
    if (index % 2 === 1) return part;
    if (STYLE_ELEMENT.test(part)) {
      throw new Error(
        "has an inline <style> element, which style-src 'self' blocks: set build.inlineStylesheets to \"never\"",
      );
    }
    return part.replace(STYLED, (_whole, name, before, double, single, after) => {
      const style = String(double ?? single ?? "");
      const tag = String(name).toLowerCase();
      const known = scripted.find((s) => s.tag === tag && s.style.trim() === style.trim());
      let className = known?.className;
      if (className === undefined) {
        className = classFor(style);
        styles.set(className, style);
      }
      return `<${name}${withClass(`${before}${after}`, className)}>`;
    });
  });
  return { html: parts.join(""), styles };
}

/**
 * Runs `step` on `file`, naming the file (from `dir`) in what it throws.
 *
 * @template T
 * @param {URL} dir
 * @param {URL} file
 * @param {() => Promise<T>} step
 * @returns {Promise<T>}
 */
async function inFile(dir, file, step) {
  try {
    return await step();
  } catch (error) {
    throw new Error(`${file.pathname.slice(dir.pathname.length)}: ${error instanceof Error ? error.message : error}`);
  }
}

/**
 * Translates the selectors on the style attribute in every style sheet under `dir`'s `_astro/`.
 *
 * @param {URL} dir the build's output folder
 * @returns {Promise<void>}
 */
async function translateStyleSheetsIn(dir) {
  for (const file of await filesIn(new URL("_astro/", dir), ".css")) {
    await inFile(dir, file, async () => {
      const css = await readFile(file, "utf8");
      const translated = translateStyleSelectors(css);
      if (translated !== css) await writeFile(file, translated);
    });
  }
}

/**
 * Writes the style sheet of `styles` into `dir`'s `_astro/`, named by its hash.
 *
 * @param {URL} dir the build's output folder
 * @param {Map<string, string>} styles each class and the style it stands for
 * @returns {Promise<string>} the style sheet's address
 */
async function writeStyleSheet(dir, styles) {
  const css = [...styles]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([className, style]) => ruleFor(className, style))
    .join("\n");
  const name = `inline-styles.${createHash("sha256").update(css).digest("hex").slice(0, 8)}.css`;
  await writeFile(new URL(`_astro/${name}`, dir), `${css}\n`);
  return `/_astro/${name}`;
}

/**
 * Moves the inline styles of every page under `dir` into one style sheet, `_astro/inline-styles.
 * <hash>.css`, which each page that had one links at the end of its head, and translates the
 * built style sheets' selectors on the style attribute.
 *
 * @param {URL} dir the build's output folder
 * @param {readonly ScriptedStyle[]} [scripted]
 * @returns {Promise<{ pages: number, classes: number }>}
 */
export async function moveInlineStylesIn(dir, scripted = []) {
  /** @type {Map<string, string>} */
  const all = new Map();
  /** @type {{ file: URL, html: string }[]} */
  const styled = [];
  for (const file of await filesIn(dir, ".html")) {
    const moved = await inFile(dir, file, async () => moveInlineStyles(await readFile(file, "utf8"), scripted));
    for (const [className, style] of moved.styles) all.set(className, style);
    if (moved.styles.size > 0) styled.push({ file, html: moved.html });
  }
  await translateStyleSheetsIn(dir);
  if (all.size === 0) return { pages: 0, classes: 0 };
  const link = `<link rel="stylesheet" href="${await writeStyleSheet(dir, all)}">`;
  for (const { file, html } of styled) {
    await inFile(dir, file, async () => {
      if (!/<\/head>/i.test(html)) throw new Error("has no </head> to link the style sheet from");
      await writeFile(file, html.replace(/<\/head>/i, `${link}</head>`));
    });
  }
  return { pages: styled.length, classes: all.size };
}

/**
 * The Astro integration: {@link moveInlineStylesIn} once the build is done.
 *
 * @param {{ scripted?: readonly ScriptedStyle[] }} [options]
 * @returns {import("astro").AstroIntegration}
 */
export function inlineStylesToClasses(options = {}) {
  return {
    name: "lamina-inline-styles-to-classes",
    hooks: {
      "astro:build:done": async ({ dir, logger }) => {
        const { pages, classes } = await moveInlineStylesIn(dir, options.scripted);
        logger.info(`moved the inline styles of ${pages} pages into ${classes} classes`);
      },
    },
  };
}
