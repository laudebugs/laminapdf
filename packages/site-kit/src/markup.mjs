// Reading built HTML the way a browser splits it, enough for the build steps' checks and rewrites:
// left to right, a comment, a <script> or <style> element with its raw text, a start tag with its
// attributes (double, single or no quotes, so a value holding `<` or `>` stays part of its tag),
// or text. It is not a full HTML parser: it trusts the build's markup to be well formed.

/** One attribute of a start tag: its lowercased name, its value as written, and its source text. */
/** @typedef {{ name: string, value: string, text: string }} Attribute */

/**
 * A start tag: its name lowercased and as written, its attributes, whether it ends in `/>`.
 *
 * @typedef {{ name: string, written: string, attributes: Attribute[], selfClosing: boolean }} StartTag
 */

/**
 * One piece of a page: a start tag, a `<script>` or `<style>` element (its start tag, with its body
 * in `text`), or anything else (a comment, an end tag, text).
 *
 * @typedef {({ kind: "start", text: string } & StartTag)
 *   | { kind: "raw", text: string, tag: StartTag }
 *   | { kind: "other", text: string }} Token
 */

const ATTRIBUTE = /\s+([^\s"'>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;

/** An attribute, without capturing its parts, for the patterns of whole tags. */
const ATTRIBUTE_TEXT = ATTRIBUTE.source.replace(/\((?!\?)/g, "(?:");

const START_TAG = `<([a-zA-Z][^\\s/>]*)((?:${ATTRIBUTE_TEXT})*)\\s*(/?)>`;

/** A start tag at the start of a string. */
const OPENING = new RegExp(`^${START_TAG}`, "i");

const TOKEN = new RegExp(
  [
    "<!--[\\s\\S]*?-->",
    "<(script|style)\\b(?:[^>\"']|\"[^\"]*\"|'[^']*')*>[\\s\\S]*?</\\1\\s*>",
    START_TAG,
    "[^<]+",
    "<",
  ].join("|"),
  "gi",
);

/**
 * The start tag `text` begins with.
 *
 * @param {string} text
 * @returns {StartTag | undefined}
 */
function startTagAt(text) {
  const match = OPENING.exec(text);
  if (match === null) return undefined;
  const [, written = "", attributes = "", slash] = match;
  return {
    name: written.toLowerCase(),
    written,
    attributes: [...attributes.matchAll(ATTRIBUTE)].map(([whole, name = "", double, single, bare]) => ({
      name: name.toLowerCase(),
      value: double ?? single ?? bare ?? "",
      text: whole,
    })),
    selfClosing: slash === "/",
  };
}

/**
 * The pieces of `html`, in order; joining their `text` gives `html` back.
 *
 * @param {string} html
 * @returns {Token[]}
 */
export function tokens(html) {
  return [...html.matchAll(TOKEN)].map(([text, raw, written]) => {
    const tag = raw !== undefined || written !== undefined ? startTagAt(text) : undefined;
    if (tag === undefined) return { kind: "other", text };
    return raw === undefined ? { kind: "start", text, ...tag } : { kind: "raw", text, tag };
  });
}

/**
 * Every start tag in `html`, `<script>` and `<style>` among them: its lowercased name and its
 * attributes by lowercased name, with their values as written.
 *
 * @param {string} html
 * @returns {{ name: string, attributes: Map<string, string> }[]}
 */
export function startTags(html) {
  return tokens(html).flatMap((token) => {
    if (token.kind === "other") return [];
    const tag = token.kind === "raw" ? token.tag : token;
    return [{ name: tag.name, attributes: new Map(tag.attributes.map((a) => [a.name, a.value])) }];
  });
}

/** The character references a built page's attribute values use. */
const NAMED = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };

/**
 * `value` with its character references (`&quot;`, `&#39;`, `&#x27;`) replaced by what they stand for.
 *
 * @param {string} value
 * @returns {string}
 */
export function decodeReferences(value) {
  return value.replace(/&(?:#(\d+)|#x([0-9a-f]+)|([a-z]+));/gi, (whole, decimal, hex, name) => {
    if (decimal !== undefined) return String.fromCodePoint(Number(decimal));
    if (hex !== undefined) return String.fromCodePoint(Number.parseInt(hex, 16));
    return NAMED[/** @type {keyof typeof NAMED} */ (String(name).toLowerCase())] ?? whole;
  });
}
