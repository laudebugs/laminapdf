// The docs site's own build steps (astro.config.mjs): the example PDFs, the checks of links the
// links validator can't see, and the CSP's inline-script hashes. Each runs after the build, on
// the files in dist/.
import { createHash } from "node:crypto";
import { access, copyFile, mkdir, readdir, readFile, writeFile } from "node:fs/promises";

/** The token in public/_headers that {@link inlineScriptHashes} replaces. */
const HASHES = "'inline-script-hashes'";

/**
 * Cloudflare's limit on one line of a `_headers` file is 2,000 characters; the CSP line must
 * stay under it, with a margin for a few more scripts.
 */
const MAX_HEADER_LINE = 1_900;

/**
 * Every HTML file under `dir` (a file: URL), recursively.
 *
 * @param {URL} dir
 * @returns {Promise<URL[]>}
 */
async function htmlFiles(dir) {
  const entries = await readdir(dir, { recursive: true });
  return entries.filter((name) => name.endsWith(".html")).map((name) => new URL(name, dir));
}

/**
 * Whether `url` names an existing file.
 *
 * @param {URL} url
 * @returns {Promise<boolean>}
 */
async function exists(url) {
  try {
    await access(url);
    return true;
  } catch {
    return false;
  }
}

/**
 * Copies the examples' golden PDFs (`goldens`, a directory URL) into the site at /examples/pdf/.
 *
 * @param {URL} goldens
 * @returns {import("astro").AstroIntegration}
 */
export function examplePdfs(goldens) {
  return {
    name: "lamina-example-pdfs",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        const out = new URL("examples/pdf/", dir);
        await mkdir(out, { recursive: true });
        for (const name of await readdir(goldens)) {
          if (name.endsWith(".pdf")) await copyFile(new URL(name, goldens), new URL(name, out));
        }
      },
    },
  };
}

/**
 * Fails the build on a link from any page to a file the build didn't write, for the paths the
 * links validator leaves out: the example PDFs (`/examples/pdf/…`, not pages) and the code pages
 * (`/reference/codes/<code>/`, written by src/pages/, which the validator doesn't read).
 *
 * @type {import("astro").AstroIntegration}
 */
export const checkedLinks = {
  name: "lamina-checked-links",
  hooks: {
    "astro:build:done": async ({ dir }) => {
      const broken = new Set();
      for (const file of await htmlFiles(dir)) {
        const html = await readFile(file, "utf8");
        for (const [, path = ""] of html.matchAll(/href="(\/(?:examples\/pdf|reference\/codes)\/[^"#?]+)/g)) {
          const target = new URL(`.${path.endsWith("/") ? `${path}index.html` : path}`, dir);
          if (!(await exists(target))) broken.add(path);
        }
      }
      if (broken.size > 0) throw new Error(`links to files the build didn't write: ${[...broken].sort().join(", ")}`);
    },
  },
};

/**
 * Puts the SHA-256 of every inline script the build wrote (Starlight's theme and tabs, Expressive
 * Code's copy button) into the CSP of dist/_headers, so `script-src` allows exactly those and
 * the site's own files: no 'unsafe-inline' (ADR 0011 §4). A script added or changed by an
 * upgrade gets its hash on the next build. Fails if the CSP line outgrows Cloudflare's limit.
 *
 * @type {import("astro").AstroIntegration}
 */
export const inlineScriptHashes = {
  name: "lamina-csp-script-hashes",
  hooks: {
    "astro:build:done": async ({ dir }) => {
      const hashes = new Set();
      for (const file of await htmlFiles(dir)) {
        const html = await readFile(file, "utf8");
        for (const [, attributes, body = ""] of html.matchAll(/<script(\s[^>]*)?>([\s\S]*?)<\/script>/g)) {
          if (/\ssrc=/.test(attributes ?? "") || body.length === 0) continue;
          hashes.add(`'sha256-${createHash("sha256").update(body, "utf8").digest("base64")}'`);
        }
      }
      const headersFile = new URL("_headers", dir);
      const headers = await readFile(headersFile, "utf8");
      if (!headers.includes(HASHES)) throw new Error(`public/_headers has no ${HASHES} to replace`);
      const filled = headers.replace(HASHES, [...hashes].sort().join(" "));
      const csp = filled.split("\n").find((line) => line.trimStart().startsWith("Content-Security-Policy:")) ?? "";
      if (csp.length > MAX_HEADER_LINE) {
        throw new Error(
          `the CSP line in dist/_headers is ${csp.length} characters (${hashes.size} inline-script hashes), over ` +
            `the ${MAX_HEADER_LINE} this build allows (Cloudflare's limit is 2,000): move inline scripts into files`,
        );
      }
      await writeFile(headersFile, filled);
    },
  },
};
