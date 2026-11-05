// Build steps that Lamina PDF's Astro sites share (the docs site and the website). Each runs after
// the build, on the files in dist/. Node-only: it runs at build time, never in a page.
import { createHash } from "node:crypto";
import { readdirSync } from "node:fs";
import { readdir, readFile, stat, writeFile } from "node:fs/promises";

/** The token in public/_headers that {@link fillInlineScriptHashes} replaces. */
const HASHES = "'inline-script-hashes'";

/**
 * Cloudflare's limit on one line of a `_headers` file is 2,000 characters; the CSP line must
 * stay under it, with a margin for a few more scripts.
 */
const MAX_HEADER_LINE = 1_900;

/**
 * Every file under `dir` (a file: URL) whose name ends in `extension`, recursively.
 *
 * @param {URL} dir
 * @param {string} extension
 * @returns {Promise<URL[]>}
 */
export async function filesIn(dir, extension) {
  const entries = await readdir(dir, { recursive: true });
  return entries.filter((name) => name.endsWith(extension)).map((name) => new URL(name, dir));
}

/**
 * The page files under `dir` (a site's src/pages folder, a file: URL), as `/src/pages/…` paths: the
 * pages the header may link to (nav.mjs). A site's astro.config.mjs reads them once per build.
 *
 * @param {URL} dir
 * @returns {string[]}
 */
export function pageFiles(dir) {
  return readdirSync(dir, { recursive: true })
    .map((file) => `/src/pages/${String(file).replaceAll("\\", "/")}`)
    .filter((file) => /\.(astro|md)$/.test(file))
    .sort();
}

/**
 * Whether `url` names an existing file (not a folder).
 *
 * @param {URL} url
 * @returns {Promise<boolean>}
 */
async function isFile(url) {
  try {
    return (await stat(url)).isFile();
  } catch {
    return false;
  }
}

/**
 * The body of every inline script in `html`: each `<script>` without a `src`, with text.
 *
 * @param {string} html
 * @returns {string[]}
 */
function inlineScripts(html) {
  const bodies = [];
  for (const [, attributes = "", body = ""] of html.matchAll(/<script(\s[^>]*)?>([\s\S]*?)<\/script>/gi)) {
    if (!/\ssrc\s*=/i.test(attributes) && body.length > 0) bodies.push(body);
  }
  return bodies;
}

/**
 * Puts the SHA-256 of every inline script the build wrote into the CSP of dist/_headers, so
 * `script-src` allows exactly those and the site's own files: no 'unsafe-inline' (ADR 0011 §4). A
 * script added or changed by an upgrade gets its hash on the next build. Fails if the CSP line
 * outgrows Cloudflare's limit.
 *
 * @param {URL} dir the build's output folder
 * @returns {Promise<void>}
 */
export async function fillInlineScriptHashes(dir) {
  const hashes = new Set();
  for (const file of await filesIn(dir, ".html")) {
    for (const body of inlineScripts(await readFile(file, "utf8"))) {
      hashes.add(`'sha256-${createHash("sha256").update(body, "utf8").digest("base64")}'`);
    }
  }
  const headersFile = new URL("_headers", dir);
  const lines = (await readFile(headersFile, "utf8")).split("\n");
  // Only a CSP line's token is replaced, never one in a comment. With no inline script, the
  // token goes with the space before it: `script-src 'self';`.
  const isCsp = (/** @type {string} */ line) => line.trimStart().startsWith("Content-Security-Policy:");
  if (!lines.some((line) => isCsp(line) && line.includes(HASHES))) {
    throw new Error(`public/_headers has no Content-Security-Policy line with ${HASHES} to replace`);
  }
  const value = [...hashes].sort().join(" ");
  const filled = lines.map((line) =>
    isCsp(line) ? (hashes.size === 0 ? line.replace(` ${HASHES}`, "") : line.replace(HASHES, value)) : line,
  );
  for (const csp of filled.filter(isCsp)) {
    if (csp.length > MAX_HEADER_LINE) {
      throw new Error(
        `the CSP line in dist/_headers is ${csp.length} characters (${hashes.size} inline-script hashes), over ` +
          `the ${MAX_HEADER_LINE} this build allows (Cloudflare's limit is 2,000): move inline scripts into files`,
      );
    }
  }
  await writeFile(headersFile, filled.join("\n"));
}

/** @type {import("astro").AstroIntegration} */
export const inlineScriptHashes = {
  name: "lamina-csp-script-hashes",
  hooks: { "astro:build:done": ({ dir }) => fillInlineScriptHashes(dir) },
};

/**
 * Adds `X-Robots-Tag: noindex` to the `/*` block of dist/_headers unless `env` is `production`.
 *
 * @param {URL} dir the build's output folder
 * @param {string | undefined} env the build's `LAMINA_PDF_ENV`
 * @returns {Promise<void>}
 */
export async function markNoindex(dir, env) {
  if (env === "production") return;
  const headersFile = new URL("_headers", dir);
  const headers = await readFile(headersFile, "utf8");
  if (!/^\/\*$/m.test(headers)) throw new Error("public/_headers has no /* block for X-Robots-Tag");
  await writeFile(headersFile, headers.replace(/^\/\*$/m, "/*\n  X-Robots-Tag: noindex"));
}

/**
 * Adds `X-Robots-Tag: noindex` to every response of a build that isn't production's, so previews
 * never reach a search index (ADR 0011 §1). Production builds set `LAMINA_PDF_ENV=production`;
 * every other build, a local one included, is noindex.
 *
 * @param {string | undefined} env the build's `LAMINA_PDF_ENV`
 * @returns {import("astro").AstroIntegration}
 */
export function previewNoindex(env) {
  return { name: "lamina-preview-noindex", hooks: { "astro:build:done": ({ dir }) => markNoindex(dir, env) } };
}

/** Elements whose address the page loads (not a link the reader follows). */
const LOADING = new Set([
  "script",
  "link",
  "img",
  "source",
  "video",
  "audio",
  "iframe",
  "embed",
  "object",
  "track",
  "input",
]);
/** The attributes that hold an address an element loads. */
const ADDRESSES = ["src", "href", "srcset", "data", "poster"];
/** `<link>` relations that only name a page, not load it. */
const NAMES_A_PAGE = /\b(canonical|alternate|me|author)\b/i;
/** A style sheet or font from another origin, in CSS: `url(https://…)`, `url(//…)`, `@import "…"`. */
const REMOTE_CSS = /(?:url\(\s*["']?|@import\s+(?:url\(\s*)?["']?)(?:https?:)?\/\//i;
/** Script that sets a cookie (`document.cookie =`, `document["cookie"] =`, the Cookie Store API). */
const COOKIE =
  /\bdocument\s*(?:\.\s*cookie|\[\s*["'`]cookie["'`]\s*\])\s*=(?!=)|\bcookieStore\s*\.\s*(?:set|delete)\s*\(/;
/** Script that sends a beacon (ADR 0011 §3: no analytics). */
const BEACON = /\bsendBeacon\s*\(/;
/** A module imported from another origin: Astro bundles `<script src="https://…">` into one. */
const REMOTE_IMPORT = /(?:\bimport\s*(?:\(\s*)?|\bfrom\s*)["'`](?:https?:)?\/\//;

/**
 * `html` with comments removed and the bodies of `<script>` elements emptied, so the markup
 * checks see only markup, never text inside a script or a comment.
 *
 * @param {string} html
 * @returns {string}
 */
function markupOf(html) {
  return html.replace(/<!--[\s\S]*?-->/g, "").replace(/(<script\b[^>]*>)[\s\S]*?<\/script>/gi, "$1</script>");
}

/**
 * Every start tag in `markup` (from {@link markupOf}): its lowercase name and attributes, with
 * names lowercased and values in double, single or no quotes.
 *
 * @param {string} markup
 * @returns {{ name: string, attributes: Map<string, string> }[]}
 */
function tags(markup) {
  return [...markup.matchAll(/<([a-z][a-z0-9-]*)\b([^>]*)>/gi)].map(([, name = "", text = ""]) => {
    const attributes = new Map();
    for (const [, key = "", ...values] of text.matchAll(
      /([^\s"'=<>/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g,
    )) {
      attributes.set(key.toLowerCase(), values.find((v) => v !== undefined) ?? "");
    }
    return { name: name.toLowerCase(), attributes };
  });
}

/**
 * Every address the page loads: `src`, `href`, `srcset`, `data` and `poster` of the elements that
 * load what they name. Links the reader follows don't count, nor do the `<link>`s that only name a
 * page (canonical, alternate, me, author), nor `data:` addresses.
 *
 * @param {{ name: string, attributes: Map<string, string> }[]} found
 * @returns {string[]}
 */
function loadedAddresses(found) {
  const addresses = [];
  for (const { name, attributes } of found) {
    if (!LOADING.has(name) || (name === "link" && NAMES_A_PAGE.test(attributes.get("rel") ?? ""))) continue;
    for (const attribute of ADDRESSES) {
      // A srcset lists `address width` pairs, separated by commas.
      const value = attributes.get(attribute) ?? "";
      addresses.push(...value.split(",").map((part) => part.trim().split(/\s+/)[0] ?? ""));
    }
  }
  return addresses.filter((address) => address !== "" && !/^data:/i.test(address));
}

/**
 * What is wrong with one built HTML page, as messages: a file it loads from another origin, or
 * from its own origin by full address (a preview's `'self'` is another host); an inline style
 * (`style-src 'self'` blocks it); a link that pings; a missing title or description; no canonical
 * URL on `origin` (unless the page is `noindex`); or an inline script that imports a module from
 * another origin, sets a cookie or sends a beacon.
 *
 * @param {string} html
 * @param {string} origin the site's production origin, such as `https://laminapdf.com`
 * @returns {string[]}
 */
export function pageProblems(html, origin) {
  const markup = markupOf(html);
  const found = tags(markup);
  return [
    ...absoluteLoads(found, origin),
    ...markupProblems(markup, found),
    ...headProblems(markup, found, origin),
    ...inlineScripts(html).flatMap(scriptProblems),
  ];
}

/**
 * The loads by full address: from another origin, or from `origin` itself, which works in
 * production but not on a preview, whose `'self'` is another host.
 *
 * @param {{ name: string, attributes: Map<string, string> }[]} found
 * @param {string} origin
 * @returns {string[]}
 */
function absoluteLoads(found, origin) {
  return loadedAddresses(found)
    .filter((address) => /^([a-z][a-z0-9+.-]*:)?\/\//i.test(address))
    .map((address) =>
      address.startsWith(`${origin}/`)
        ? `loads ${address} by its full address, which a preview's CSP refuses: use the path`
        : `loads ${address} from another origin`,
    );
}

/**
 * What the markup does that the website refuses: an inline style (`style-src 'self'` blocks it)
 * or a link that pings.
 *
 * @param {string} markup
 * @param {{ name: string, attributes: Map<string, string> }[]} found
 * @returns {string[]}
 */
function markupProblems(markup, found) {
  const inlineStyle = /<style[\s>]/i.test(markup) || found.some(({ attributes }) => attributes.has("style"));
  const ping = found.some(({ name, attributes }) => name === "a" && attributes.has("ping"));
  return [
    ...(inlineStyle ? ["has an inline style, which style-src 'self' blocks"] : []),
    ...(ping ? ["has a link with a ping"] : []),
  ];
}

/**
 * What the head lacks: a title, a description, or a canonical URL on `origin`. A page marked
 * `<meta name="robots" content="noindex">`, such as the 404 page, needs no canonical URL.
 *
 * @param {string} markup
 * @param {{ name: string, attributes: Map<string, string> }[]} found
 * @param {string} origin
 * @returns {string[]}
 */
function headProblems(markup, found, origin) {
  const meta = (/** @type {string} */ key) =>
    found
      .find(({ name, attributes }) => name === "meta" && attributes.get("name")?.toLowerCase() === key)
      ?.attributes.get("content") ?? "";
  const canonical = found.find(
    ({ name, attributes }) => name === "link" && /\bcanonical\b/i.test(attributes.get("rel") ?? ""),
  );
  const noindex = /\bnoindex\b/i.test(meta("robots"));
  return [
    ...(/<title>[^<]*\S[^<]*<\/title>/.test(markup) ? [] : ["has no <title>"]),
    ...(meta("description").trim() ? [] : ["has no description"]),
    ...(noindex || canonical?.attributes.get("href")?.startsWith(`${origin}/`)
      ? []
      : [`has no canonical URL on ${origin}`]),
  ];
}

/**
 * What is wrong with one script: it imports a module from another origin, sets a cookie or sends
 * a beacon.
 *
 * @param {string} text
 * @returns {string[]}
 */
function scriptProblems(text) {
  return [
    ...(REMOTE_IMPORT.test(text) ? ["imports a module from another origin"] : []),
    ...(COOKIE.test(text) ? ["sets a cookie"] : []),
    ...(BEACON.test(text) ? ["sends a beacon"] : []),
  ];
}

/**
 * Fails the build when it breaks the website's rules (ADR 0011 §3): a page loads a file from
 * another origin, by its full address, or one the build didn't write; links to a page of this
 * site the build didn't write; has an inline style or a link that pings; or lacks a title, a
 * description or a canonical URL on `origin`. It also fails when a script imports a module from
 * another origin, sets a cookie or sends a beacon; a style sheet loads a file from another origin;
 * or `_headers` sets a cookie.
 *
 * @param {{ origin: string }} options
 * @returns {import("astro").AstroIntegration}
 */
export function siteChecks({ origin }) {
  return {
    name: "lamina-site-checks",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const problems = await builtSiteProblems(dir, origin);
        if (problems.length > 0) throw new Error(`the built site breaks its rules:\n  ${problems.join("\n  ")}`);
      },
    },
  };
}

/**
 * Every problem {@link siteChecks} reports for the build in `dir`, as `path: message` lines.
 *
 * @param {URL} dir
 * @param {string} origin
 * @returns {Promise<string[]>}
 */
export async function builtSiteProblems(dir, origin) {
  const problems = [];
  const relative = (/** @type {URL} */ url) => url.pathname.slice(dir.pathname.length);
  for (const file of await filesIn(dir, ".html")) {
    const html = await readFile(file, "utf8");
    const page = relative(file);
    for (const problem of pageProblems(html, origin)) problems.push(`${page}: ${problem}`);
    for (const problem of await missingTargets(html, page, dir, origin)) problems.push(`${page}: ${problem}`);
  }
  for (const file of await filesIn(dir, ".js")) {
    for (const problem of scriptProblems(await readFile(file, "utf8"))) problems.push(`${relative(file)}: ${problem}`);
  }
  for (const file of await filesIn(dir, ".css")) {
    if (REMOTE_CSS.test(await readFile(file, "utf8")))
      problems.push(`${relative(file)}: loads a file from another origin`);
  }
  if (/^\s*set-cookie\s*:/im.test(await readFile(new URL("_headers", dir), "utf8"))) {
    problems.push("_headers: sets a cookie");
  }
  return problems;
}

/**
 * The links and loads of the page at `page` (its path in `dir`) whose target on `origin` the build
 * didn't write. Each address resolves as a browser would against the page's own address, so a
 * relative (`pricing/`), root (`/pricing/`) or full (`https://laminapdf.com/pricing/`) address
 * all count; addresses on other hosts and in other schemes don't.
 *
 * @param {string} html
 * @param {string} page
 * @param {URL} dir
 * @param {string} origin
 * @returns {Promise<string[]>}
 */
async function missingTargets(html, page, dir, origin) {
  const found = tags(markupOf(html));
  const base = new URL(page, `${origin}/`);
  const links = found.filter(({ name }) => name === "a").map(({ attributes }) => attributes.get("href") ?? "");
  const missing = [];
  for (const [kind, addresses] of /** @type {const} */ ([
    ["links to", links],
    ["loads", loadedAddresses(found)],
  ])) {
    for (const address of addresses) {
      if (address === "" || address.startsWith("#")) continue;
      const url = URL.parse(address, base);
      if (url?.origin !== origin || (await pageExists(dir, url.pathname))) continue;
      missing.push(`${kind} missing ${url.pathname}`);
    }
  }
  return missing;
}

/**
 * Whether the build in `dir` serves `path` (a URL path such as `/privacy/`): the file itself,
 * the folder's `index.html`, or `path.html`, the three forms Workers static assets answer with
 * `html_handling = "auto-trailing-slash"`.
 *
 * @param {URL} dir
 * @param {string} path
 * @returns {Promise<boolean>}
 */
export async function pageExists(dir, path) {
  const at = (/** @type {string} */ p) => isFile(new URL(`.${p}`, dir));
  if (path.endsWith("/")) return at(`${path}index.html`);
  return (await at(path)) || (await at(`${path}/index.html`)) || at(`${path}.html`);
}
