// Build steps that Lamina PDF's Astro sites share (the docs site and the website). Each runs after
// the build, on the files in dist/. Node-only: it runs at build time, never in a page.
import { createHash } from "node:crypto";
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
 * Whether `url` names an existing file (not a folder).
 *
 * @param {URL} url
 * @returns {Promise<boolean>}
 */
export async function isFile(url) {
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

/** Elements whose address the page loads (not a link the reader follows), with their attributes. */
const LOADED = /<(script|link|img|source|video|audio|iframe|embed|object|track|input)\b([^>]*)>/gi;
const ADDRESS = /\s(src|href|srcset|data|poster)\s*=\s*["']([^"']+)["']/gi;
/** A style sheet or font from another origin, in CSS: `url(https://…)`, `url(//…)`, `@import "…"`. */
const REMOTE_CSS = /(?:url\(\s*["']?|@import\s+(?:url\(\s*)?["']?)(?:https?:)?\/\//i;
/** Script that sets a cookie or sends a beacon (ADR 0011 §3: no cookies, no analytics). */
const COOKIE = /document\.cookie\s*=/;
const BEACON = /\bsendBeacon\s*\(/;
/** A module imported from another origin: Astro bundles `<script src="https://…">` into one. */
const REMOTE_IMPORT = /(?:\bimport\s*(?:\(\s*)?|\bfrom\s*)["'`](?:https?:)?\/\//;

/**
 * What is wrong with one built HTML page, as messages: a file it loads from another origin, an
 * inline style (`style-src 'self'` blocks it), a missing title, description or canonical URL on
 * `origin`, a ping on a link, or an inline script that sets a cookie or sends a beacon.
 *
 * @param {string} html
 * @param {string} origin the site's production origin, such as `https://laminapdf.com`
 * @returns {string[]}
 */
export function pageProblems(html, origin) {
  return [
    ...remoteLoads(html, origin).map((address) => `loads ${address} from another origin`),
    ...markupProblems(html),
    ...headProblems(html, origin),
    ...inlineScripts(html).flatMap(scriptProblems),
  ];
}

/**
 * Every address `html` loads: `src`, `href`, `srcset`, `data` and `poster` of the elements that
 * load what they name. Links the reader follows don't count, nor do the `<link>`s that only name a
 * page (canonical, alternate, me, author).
 *
 * @param {string} html
 * @returns {string[]}
 */
function loadedAddresses(html) {
  const addresses = [];
  for (const [, tag = "", attributes = ""] of html.matchAll(LOADED)) {
    const relation = attributes.match(/\srel\s*=\s*["']([^"']+)["']/i)?.[1] ?? "";
    if (tag.toLowerCase() === "link" && /\b(canonical|alternate|me|author)\b/i.test(relation)) continue;
    for (const [, , value = ""] of attributes.matchAll(ADDRESS)) {
      // A srcset lists `address width` pairs, separated by commas.
      addresses.push(...value.split(",").map((part) => part.trim().split(/\s+/)[0] ?? ""));
    }
  }
  return addresses;
}

/**
 * The addresses `html` loads from an origin other than `origin`.
 *
 * @param {string} html
 * @param {string} origin
 * @returns {string[]}
 */
function remoteLoads(html, origin) {
  return loadedAddresses(html).filter((a) => /^(https?:)?\/\//i.test(a) && !a.startsWith(`${origin}/`));
}

/**
 * What the markup of `html` does that the website refuses: an inline style (`style-src 'self'`
 * blocks it) or a link that pings.
 *
 * @param {string} html
 * @returns {string[]}
 */
function markupProblems(html) {
  const inlineStyle = /<style[\s>]/i.test(html) || /<[a-z][^>]*\sstyle\s*=/i.test(html);
  return [
    ...(inlineStyle ? ["has an inline style, which style-src 'self' blocks"] : []),
    ...(/<a\s[^>]*\bping\s*=/i.test(html) ? ["has a link with a ping"] : []),
  ];
}

/**
 * What the head of `html` lacks: a title, a description or a canonical URL on `origin`.
 *
 * @param {string} html
 * @param {string} origin
 * @returns {string[]}
 */
function headProblems(html, origin) {
  const canonical = html.match(/<link\s+rel="canonical"\s+href="([^"]+)"/)?.[1];
  return [
    ...(/<title>[^<]+<\/title>/.test(html) ? [] : ["has no <title>"]),
    ...(/<meta\s+name="description"\s+content="[^"]+"/.test(html) ? [] : ["has no description"]),
    ...(canonical?.startsWith(`${origin}/`) ? [] : [`has no canonical URL on ${origin}`]),
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
 * another origin or one the build didn't write, links to a page the build didn't write, has an
 * inline style or a link that pings, or lacks a title, a description or a canonical URL on
 * `origin`; a script imports a module from another origin, sets a cookie or sends a beacon; a
 * style sheet loads a file from another origin; or `_headers` sets a cookie.
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
    for (const problem of pageProblems(html, origin)) problems.push(`${relative(file)}: ${problem}`);
    for (const path of await missingLinks(html, dir)) problems.push(`${relative(file)}: links to missing ${path}`);
    for (const path of await missingLoads(html, dir)) problems.push(`${relative(file)}: loads missing ${path}`);
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
 * The site paths `html` links to (`<a href="/…">`) that the build in `dir` didn't write, either
 * as the file itself or as a page (`index.html`).
 *
 * @param {string} html
 * @param {URL} dir
 * @returns {Promise<string[]>}
 */
async function missingLinks(html, dir) {
  const missing = [];
  for (const [, path = ""] of html.matchAll(/<a\s[^>]*href="(\/(?!\/)[^"#?]*)/g)) {
    const page = new URL(`.${path.replace(/\/?$/, "/")}index.html`, dir);
    if (!(await isFile(new URL(`.${path}`, dir))) && !(await isFile(page))) missing.push(path);
  }
  return missing;
}

/**
 * The site paths `html` loads (a script, style sheet, font or image at `/…`) that the build in
 * `dir` didn't write.
 *
 * @param {string} html
 * @param {URL} dir
 * @returns {Promise<string[]>}
 */
async function missingLoads(html, dir) {
  const missing = [];
  for (const address of loadedAddresses(html)) {
    const path = address.split(/[?#]/)[0] ?? "";
    if (/^\/(?!\/)/.test(path) && !(await isFile(new URL(`.${path}`, dir)))) missing.push(path);
  }
  return missing;
}
