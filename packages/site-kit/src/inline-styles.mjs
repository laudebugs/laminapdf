// Inline styles moved into a style sheet after the build, so a site's CSP needs no 'unsafe-inline'
// in style-src (ADR 0011 §4). Expressive Code colours each code token with a style attribute, and
// Starlight writes a few of its own (the sidebar's depth, an icon's size, the search dialog's
// padding). Each distinct value becomes one class in a style sheet every page with one links. Its
// declarations are !important, so they still win over the site's style sheets, as the attribute did.
// A style sheet that selects elements by their style attribute stops matching once it is gone, so
// the built style sheets' one such selector, Expressive Code's for a token's colour, is rewritten
// to the class, and any other fails the build. Node-only: it runs at build time, never in a page.
import { createHash } from "node:crypto";
import { readFile, rm, writeFile } from "node:fs/promises";
import { filesIn } from "./integrations.mjs";
import { decodeReferences, tokens } from "./markup.mjs";

/**
 * An inline style a page's own script removes later. A class rule can't be removed that way, so
 * the element gets `className` instead, which the site's CSS styles for as long as the script
 * would have kept the style.
 *
 * @typedef {{ tag: string, style: string, className: string }} ScriptedStyle
 */

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

/** A quoted string, with escapes. */
const QUOTED = String.raw`"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'`;

/** A declaration: anything but a semicolon, with quoted strings and parentheses (two deep) whole. */
const DECLARATION = new RegExp(String.raw`(?:[^;"'()]|${QUOTED}|\((?:[^()"']|${QUOTED}|\([^()]*\))*\))+`, "g");

/**
 * The declarations of `style`, split at the semicolons outside quotes and parentheses (a
 * `data:` URL holds one).
 *
 * @param {string} style
 * @returns {string[]}
 */
function declarationsOf(style) {
  return [...style.matchAll(DECLARATION)].map(([declaration]) => declaration.trim()).filter((d) => d !== "");
}

/**
 * The rule that gives `className` what `style` declares, each declaration !important.
 *
 * @param {string} className
 * @param {string} style the attribute's value, its character references decoded
 * @returns {string}
 */
export function ruleFor(className, style) {
  const declarations = declarationsOf(style).map((d) => `${d.replace(/\s*!\s*important$/i, "")} !important`);
  return `.${className}{${declarations.join(";")}}`;
}

/**
 * A start tag rewritten: its style attribute dropped and `className` added to its class.
 *
 * @param {import("./markup.mjs").StartTag} tag
 * @param {string} className
 * @returns {string}
 */
function withClass(tag, className) {
  let classed = false;
  const attributes = tag.attributes.flatMap((attribute) => {
    if (attribute.name === "style") return [];
    if (attribute.name !== "class") return [attribute.text];
    classed = true;
    const value = attribute.value.trim();
    return [` class="${value === "" ? className : `${value} ${className}`}"`];
  });
  if (!classed) attributes.push(` class="${className}"`);
  return `<${tag.written}${attributes.join("")}${tag.selfClosing ? " /" : ""}>`;
}

/**
 * `html` with each style attribute replaced by a class, and the styles those classes stand for.
 * Text in comments, scripts and other attributes' values is left alone. Throws on an inline
 * `<style>` element, which the CSP would block too.
 *
 * @param {string} html
 * @param {readonly ScriptedStyle[]} [scripted]
 * @returns {{ html: string, styles: Map<string, string> }}
 */
export function moveInlineStyles(html, scripted = []) {
  /** @type {Map<string, string>} */
  const styles = new Map();
  const parts = tokens(html).map((token) => {
    if (token.kind === "raw" && token.tag.name === "style") {
      throw new Error(
        "has an inline <style> element, which style-src 'self' blocks: set build.inlineStylesheets to \"never\"",
      );
    }
    if (token.kind !== "start") return token.text;
    const attribute = token.attributes.find((a) => a.name === "style");
    if (attribute === undefined) return token.text;
    const style = decodeReferences(attribute.value).trim();
    const known = scripted.find((s) => s.tag === token.name && s.style.trim() === style);
    let className = known?.className;
    if (className === undefined) {
      className = classFor(style);
      styles.set(className, style);
    }
    return withClass(token, className);
  });
  return { html: parts.join(""), styles };
}

/**
 * Whether `html` still has an inline style: a style attribute or a `<style>` element.
 *
 * @param {string} html
 * @returns {boolean}
 */
export function hasInlineStyle(html) {
  return tokens(html).some(
    (token) =>
      (token.kind === "raw" && token.tag.name === "style") ||
      (token.kind === "start" && token.attributes.some((a) => a.name === "style")),
  );
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
 * A sheet that changes gets a new name: the old one is content-hashed, and a reader's browser keeps
 * it for a year (public/_headers), so a sheet changed under the same name would never reach them.
 *
 * @param {URL} dir the build's output folder
 * @returns {Promise<Map<string, string>>} each renamed sheet's old address and its new one
 */
async function translateStyleSheetsIn(dir) {
  /** @type {Map<string, string>} */
  const renamed = new Map();
  for (const file of await filesIn(new URL("_astro/", dir), ".css")) {
    await inFile(dir, file, async () => {
      const css = await readFile(file, "utf8");
      const translated = translateStyleSelectors(css);
      if (translated === css) return;
      const old = file.pathname.slice(dir.pathname.length - 1);
      const hash = createHash("sha256").update(translated).digest("hex").slice(0, 8);
      const address = old.replace(/\.css$/, `.${hash}.css`);
      await writeFile(new URL(address.slice(1), dir), translated);
      await rm(file);
      renamed.set(old, address);
    });
  }
  return renamed;
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
 * `html` with the renamed style sheets' addresses replaced, and `link` at the end of its head.
 *
 * @param {string} html
 * @param {Map<string, string>} renamed
 * @param {string | undefined} link
 * @returns {string}
 */
function relinked(html, renamed, link) {
  let out = html;
  for (const [old, address] of renamed) out = out.replaceAll(`"${old}"`, `"${address}"`);
  if (link === undefined) return out;
  if (!/<\/head>/i.test(out)) throw new Error("has no </head> to link the style sheet from");
  return out.replace(/<\/head>/i, `${link}</head>`);
}

/**
 * Moves the inline styles of every page under `dir` into one style sheet, `_astro/inline-styles.
 * <hash>.css`, which each page that had one links at the end of its head, and translates the
 * built style sheets' selectors on the style attribute. Throws if a page still has an inline style.
 *
 * @param {URL} dir the build's output folder
 * @param {readonly ScriptedStyle[]} [scripted]
 * @returns {Promise<{ pages: number, classes: number }>}
 */
export async function moveInlineStylesIn(dir, scripted = []) {
  /** @type {Map<string, string>} */
  const all = new Map();
  /** @type {{ file: URL, before: string, html: string, styled: boolean }[]} */
  const pages = [];
  for (const file of await filesIn(dir, ".html")) {
    const before = await readFile(file, "utf8");
    const moved = await inFile(dir, file, async () => moveInlineStyles(before, scripted));
    for (const [className, style] of moved.styles) all.set(className, style);
    pages.push({ file, before, html: moved.html, styled: moved.styles.size > 0 });
  }
  const renamed = await translateStyleSheetsIn(dir);
  const link = all.size > 0 ? `<link rel="stylesheet" href="${await writeStyleSheet(dir, all)}">` : undefined;
  for (const { file, before, html, styled } of pages) {
    await inFile(dir, file, async () => {
      const after = relinked(html, renamed, styled ? link : undefined);
      if (hasInlineStyle(after)) throw new Error("still has an inline style, which style-src 'self' blocks");
      if (after !== before) await writeFile(file, after);
    });
  }
  return { pages: pages.filter((page) => page.styled).length, classes: all.size };
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
