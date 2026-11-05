// The shared build steps on small built sites written to a temporary folder: each check passes a
// good page and names what is wrong with a bad one.

import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  builtSiteProblems,
  fillInlineScriptHashes,
  markNoindex,
  pageExists,
  pageProblems,
} from "../src/integrations.mjs";

const ORIGIN = "https://laminapdf.com";
const HEADERS = `/*\n  Content-Security-Policy: default-src 'none'; script-src 'self' 'inline-script-hashes'; style-src 'self'\n  Referrer-Policy: no-referrer\n`;

/** A page with everything the checks require and nothing they refuse, with `body` in it. */
function page(body = "", head = ""): string {
  return [
    "<!doctype html><html><head>",
    "<title>Lamina PDF</title>",
    '<meta name="description" content="PDFs from code">',
    `<link rel="canonical" href="${ORIGIN}/">`,
    '<link rel="stylesheet" href="/_astro/site.css">',
    head,
    `</head><body>${body}</body></html>`,
  ].join("");
}

const folders: string[] = [];

/** A built site: `files` (path → text) and the pages' style sheet under a fresh folder, as a dist/ URL. */
async function site(files: Record<string, string>): Promise<URL> {
  const root = await mkdtemp(join(tmpdir(), "site-kit-"));
  folders.push(root);
  for (const [path, text] of Object.entries({ _headers: HEADERS, "_astro/site.css": "", ...files })) {
    const file = join(root, path);
    await mkdir(join(file, ".."), { recursive: true });
    await writeFile(file, text);
  }
  return pathToFileURL(`${root}/`);
}

afterEach(async () => {
  await Promise.all(folders.splice(0).map((f) => rm(f, { recursive: true, force: true })));
});

describe("pageProblems", () => {
  it("passes a page that loads only its own files", () => {
    const html = page(
      '<img src="/logo.svg" alt=""><img srcset="/a.png 1x, /b.png 2x" alt=""><a href="https://github.com/">out</a>',
      '<link rel="preload" href="/_astro/sans.woff2" as="font"><script type="module" src="/_astro/x.js"></script>',
    );
    expect(pageProblems(html, ORIGIN)).toEqual([]);
  });

  it("names a file loaded from the site's own origin by its full address, which a preview refuses", () => {
    expect(pageProblems(page("", `<link rel="preload" href="${ORIGIN}/_astro/sans.woff2" as="font">`), ORIGIN)).toEqual(
      [`loads ${ORIGIN}/_astro/sans.woff2 by its full address, which a preview's CSP refuses: use the path`],
    );
  });

  it("reads attributes in any order and quoting", () => {
    const html = [
      "<html><head><title>Lamina PDF</title>",
      "<meta content='PDFs from code' name=description>",
      `<link href='${ORIGIN}/' rel="canonical">`,
      "</head><body><img alt='' SRC='https://cdn.example/x.png'></body></html>",
    ].join("");
    expect(pageProblems(html, ORIGIN)).toEqual(["loads https://cdn.example/x.png from another origin"]);
  });

  it("ignores markup inside scripts and comments", () => {
    const script = `<script type="application/ld+json">{"x": "<p style='a'>"}</script>`;
    expect(
      pageProblems(page(`${script}<!-- <style>p{}</style> <img src="https://cdn.example/x.png"> -->`), ORIGIN),
    ).toEqual([]);
  });

  it("lets a noindex page go without a canonical URL", () => {
    const notFound = page().replace(
      `<link rel="canonical" href="${ORIGIN}/">`,
      '<meta name="robots" content="noindex">',
    );
    expect(pageProblems(notFound, ORIGIN)).toEqual([]);
  });

  it("names each way a script sets a cookie, but not a comparison", () => {
    const script = (body: string) => pageProblems(page(`<script>${body}</script>`), ORIGIN);
    expect(script('if (document.cookie == "") {}')).toEqual([]);
    expect(script('document["cookie"] = "id=1"')).toEqual(["sets a cookie"]);
    expect(script('cookieStore.set("id", "1")')).toEqual(["sets a cookie"]);
  });

  it("names a script, font, image or frame from another origin", () => {
    const html = page(
      '<img srcset="/a.png 1x, https://cdn.example/b.png 2x" alt=""><iframe src="//frames.example/x"></iframe>',
      '<script src="https://cdn.example/analytics.js"></script><link rel="stylesheet" href="https://fonts.example/css">',
    );
    expect(pageProblems(html, ORIGIN)).toEqual([
      "loads https://cdn.example/analytics.js from another origin",
      "loads https://fonts.example/css from another origin",
      "loads https://cdn.example/b.png from another origin",
      "loads //frames.example/x from another origin",
    ]);
  });

  it("names a cookie or a beacon set by an inline script, but not one quoted in the text", () => {
    const html = page(
      "<p>We never write document.cookie = anything.</p><script>document.cookie = 'id=1'; navigator.sendBeacon('/x')</script>",
    );
    expect(pageProblems(html, ORIGIN)).toEqual(["sets a cookie", "sends a beacon"]);
  });

  it("names an inline style and a link with a ping", () => {
    expect(pageProblems(page('<p style="color: red">x</p>'), ORIGIN)).toEqual([
      "has an inline style, which style-src 'self' blocks",
    ]);
    expect(pageProblems(page("", "<style>p{}</style>"), ORIGIN)).toEqual([
      "has an inline style, which style-src 'self' blocks",
    ]);
    expect(pageProblems(page('<a href="/" ping="/count">x</a>'), ORIGIN)).toEqual(["has a link with a ping"]);
  });

  it("names a missing title, description or canonical URL", () => {
    expect(pageProblems("<html><head></head><body></body></html>", ORIGIN)).toEqual([
      "has no <title>",
      "has no description",
      `has no canonical URL on ${ORIGIN}`,
    ]);
    const elsewhere = page().replace(`${ORIGIN}/`, "https://docs.laminapdf.com/");
    expect(pageProblems(elsewhere, ORIGIN)).toEqual([`has no canonical URL on ${ORIGIN}`]);
  });
});

describe("builtSiteProblems", () => {
  it("passes a site whose links all resolve", async () => {
    const dir = await site({
      "index.html": page(
        '<a href="/">home</a><a href="/privacy/">p</a><a href="/privacy">p</a><a href="/keys/k.asc">k</a>',
      ),
      "privacy/index.html": page(),
      "keys/k.asc": "key",
      "_astro/site.css": "body { font-family: 'Lamina Sans'; src: url(/_astro/sans.woff2) }",
      "_astro/x.js": "export const x = 1;",
    });
    expect(await builtSiteProblems(dir, ORIGIN)).toEqual([]);
  });

  it("resolves relative and full addresses on the site, and skips other hosts and schemes", async () => {
    const links = [
      '<a href="pricing/">relative</a>',
      `<a href="${ORIGIN}/download/">full</a>`,
      '<a href="../">up</a>',
      '<a href="/404">not found</a>',
      '<a href="mailto:hello@example.com">mail</a>',
      '<a href="https://docs.laminapdf.com/x/">docs</a>',
      '<a href="#top">top</a>',
    ].join("");
    const dir = await site({ "privacy/index.html": page(links), "index.html": page(), "404.html": page() });
    expect(await builtSiteProblems(dir, ORIGIN)).toEqual([
      "privacy/index.html: links to missing /privacy/pricing/",
      "privacy/index.html: links to missing /download/",
    ]);
  });

  it("names a link to a page the build didn't write, and a folder without a page", async () => {
    const dir = await site({
      "index.html": page('<a href="/pricing/">x</a><a href="/keys">y</a><a href="//example.com/">z</a>'),
      "keys/k.asc": "key",
    });
    expect(await builtSiteProblems(dir, ORIGIN)).toEqual([
      "index.html: links to missing /pricing/",
      "index.html: links to missing /keys",
    ]);
  });

  it("names a file a page loads that the build didn't write", async () => {
    const dir = await site({
      "index.html": page(
        '<img src="/logo.svg?v=2" alt="">',
        '<link rel="preload" href="/_astro/gone.woff2" as="font">',
      ),
      "logo.svg": "<svg/>",
    });
    expect(await builtSiteProblems(dir, ORIGIN)).toEqual(["index.html: loads missing /_astro/gone.woff2"]);
  });

  it("names a script or style sheet the build wrote that breaks the rules", async () => {
    const dir = await site({
      "index.html": page(),
      "_astro/a.js": "document.cookie = 'id=1'",
      "_astro/b.js": 'import"https://cdn.example/a.js";import("//cdn.example/b.js");',
      "_astro/c.js": 'import { x } from "./x.js"; fetch("/data.json");',
      // Built from parts so @laminapdf/fonts' font-CDN guard doesn't flag this file.
      "_astro/site.css": `@import url('${"https"}://fonts.example/css');`,
      _headers: `${HEADERS}  Set-Cookie: id=1\n`,
    });
    expect(await builtSiteProblems(dir, ORIGIN)).toEqual([
      "_astro/a.js: sets a cookie",
      "_astro/b.js: imports a module from another origin",
      "_astro/site.css: loads a file from another origin",
      "_headers: sets a cookie",
    ]);
  });

  it("checks every page, each against the same rules", async () => {
    const remote = '<img src="https://cdn.example/x.png" alt="">';
    const dir = await site({ "a/index.html": page(remote), "b/index.html": page(remote) });
    const problems = await builtSiteProblems(dir, ORIGIN);
    expect(problems.sort()).toEqual([
      "a/index.html: loads https://cdn.example/x.png from another origin",
      "b/index.html: loads https://cdn.example/x.png from another origin",
    ]);
  });
});

describe("pageExists", () => {
  it("serves a file, a folder's index.html, or path.html, as auto-trailing-slash does", async () => {
    const dir = await site({ "404.html": "", "privacy/index.html": "", "keys/k.asc": "" });
    const served = async (path: string) => [path, await pageExists(dir, path)];
    expect(
      await Promise.all(["/404", "/privacy", "/privacy/", "/keys/k.asc", "/keys", "/keys/", "/x"].map(served)),
    ).toEqual([
      ["/404", true],
      ["/privacy", true],
      ["/privacy/", true],
      ["/keys/k.asc", true],
      ["/keys", false],
      ["/keys/", false],
      ["/x", false],
    ]);
  });
});

describe("fillInlineScriptHashes", () => {
  const sha256 = (text: string) => `'sha256-${createHash("sha256").update(text).digest("base64")}'`;

  it("lists the hash of each inline script once, sorted, and skips scripts with a src", async () => {
    const a = "console.log(1)";
    const b = "document.documentElement.dataset.theme = 'dark'";
    const dir = await site({
      "index.html": page(`<script>${a}</script><script>${b}</script><script src="/x.js"></script>`),
      "other/index.html": page(`<script>${a}</script>`),
    });
    await fillInlineScriptHashes(dir);
    const headers = await readFile(new URL("_headers", dir), "utf8");
    expect(headers).toContain(`script-src 'self' ${[sha256(a), sha256(b)].sort().join(" ")}; style-src`);
  });

  it("removes the token when the build wrote no inline script", async () => {
    const dir = await site({ "index.html": page() });
    await fillInlineScriptHashes(dir);
    expect(await readFile(new URL("_headers", dir), "utf8")).toContain("script-src 'self'; style-src 'self'");
  });

  it("leaves the token alone where a comment names it", async () => {
    const comment = "# The build replaces 'inline-script-hashes' below.\n";
    const dir = await site({ "index.html": page(), _headers: comment + HEADERS });
    await fillInlineScriptHashes(dir);
    expect(await readFile(new URL("_headers", dir), "utf8")).toBe(
      comment + HEADERS.replace(" 'inline-script-hashes'", ""),
    );
  });

  it("fails without the token, and when the CSP line outgrows Cloudflare's limit", async () => {
    const plain = await site({ "index.html": page(), _headers: "/*\n  Referrer-Policy: no-referrer\n" });
    await expect(fillInlineScriptHashes(plain)).rejects.toThrow("no Content-Security-Policy line with");
    const scripts = Array.from({ length: 40 }, (_, i) => `<script>f(${i})</script>`).join("");
    const many = await site({ "index.html": page(scripts) });
    await expect(fillInlineScriptHashes(many)).rejects.toThrow("40 inline-script hashes");
  });
});

describe("markNoindex", () => {
  it("adds noindex to the /* block of every build but production's", async () => {
    for (const env of [undefined, "preview"]) {
      const dir = await site({});
      await markNoindex(dir, env);
      expect(await readFile(new URL("_headers", dir), "utf8")).toMatch(/^\/\*\n {2}X-Robots-Tag: noindex\n/);
    }
    const production = await site({});
    await markNoindex(production, "production");
    expect(await readFile(new URL("_headers", production), "utf8")).toBe(HEADERS);
  });
});
