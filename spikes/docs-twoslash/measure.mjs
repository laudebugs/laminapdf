// Measures one built docs page in Chromium, Firefox and WebKit, under two CSPs: the docs' current
// policy (read from the build's _headers) and Q4i's target, the same policy with no
// 'unsafe-inline' in style-src, in light and dark themes. Prints one JSON report on stdout; with
// SHOTS=<dir>, also saves a screenshot of the first open hover per browser, policy and theme.
//
// node measure.mjs <dist-dir> [page-path]   (default page: /start/first-pdf/)
//
// Run from the repository root, so `playwright` resolves to the workspace's copy.
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { gzipSync } from "node:zlib";
import { chromium, firefox, webkit } from "playwright";

const [dist, page = "/start/first-pdf/"] = process.argv.slice(2);
if (dist === undefined) throw new Error("usage: node measure.mjs <dist-dir> [page-path]");

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".woff2": "font/woff2",
  ".png": "image/png",
  ".wasm": "application/wasm",
};

/** The site's CSP for pages, from dist/_headers (the `/*` block). */
async function currentCsp() {
  const headers = await readFile(join(dist, "_headers"), "utf8");
  const line = headers.split("\n").find((l) => l.trim().startsWith("Content-Security-Policy:"));
  if (line === undefined) throw new Error("no Content-Security-Policy in _headers");
  return line.trim().slice("Content-Security-Policy:".length).trim();
}

/** A static server for `dist` that sends `csp` with every HTML response. */
async function serve(csp) {
  const server = createServer(async (req, res) => {
    let path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname));
    if (path.endsWith("/")) path += "index.html";
    const file = join(dist, path);
    try {
      if (!(await stat(file)).isFile()) throw new Error("not a file");
      const type = TYPES[extname(file)] ?? "application/octet-stream";
      res.setHeader("Content-Type", type);
      if (type.startsWith("text/html")) res.setHeader("Content-Security-Policy", csp);
      res.end(await readFile(file));
    } catch {
      res.statusCode = 404;
      res.end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { server, origin: `http://127.0.0.1:${server.address().port}` };
}

/** What the built HTML holds inline. */
function htmlStats(html) {
  return {
    styleAttributes: (html.match(/\sstyle="/g) ?? []).length,
    inlineStyleBlocks: (html.match(/<style[\s>]/g) ?? []).length,
    inlineScriptBlocks: (html.match(/<script(?![^>]*\ssrc=)[^>]*>/g) ?? []).length,
    htmlBytes: Buffer.byteLength(html),
    htmlGzip: gzipSync(html).length,
  };
}

/** Loads the page in one browser and records what it fetched, CSP violations and the hovers. */
async function visit(engine, origin, theme, shot) {
  const browser = await engine.launch();
  const context = await browser.newContext({ hasTouch: true, deviceScaleFactor: 2 });
  const tab = await context.newPage();
  await tab.addInitScript(() => {
    globalThis.__violations = [];
    document.addEventListener("securitypolicyviolation", (e) => {
      globalThis.__violations.push(`${e.effectiveDirective} ${e.blockedURI || "inline"}`);
    });
  });
  const otherOrigins = new Set();
  tab.on("request", (r) => {
    const url = new URL(r.url());
    if (url.origin !== origin && url.protocol.startsWith("http")) otherOrigins.add(url.origin);
  });
  await tab.goto(origin + page, { waitUntil: "networkidle" });
  if (theme === "dark") {
    await tab.evaluate(() => localStorage.setItem("starlight-theme", "dark"));
    await tab.reload({ waitUntil: "networkidle" });
  }
  const hovers = await hoverChecks(tab, shot);
  const violations = await tab.evaluate(() => [...new Set(globalThis.__violations)]);
  await browser.close();
  return { otherOrigins: [...otherOrigins], violations, hovers };
}

/** Whether a hover's type shows by mouse, by keyboard focus and by touch, and what it exposes. */
async function hoverChecks(tab, shot) {
  const tokens = await tab.locator(".twoslash-hover").count();
  if (tokens === 0) return { tokens: 0 };
  // The first hover with a type in it (some hovers are only a highlight).
  const token = tab.locator(".twoslash-hover:has(.twoslash-popup-container)").first();
  // The plugin's script may show a copy of the popup elsewhere in the page, so any open popup counts.
  const shown = () =>
    tab.evaluate(() =>
      [...document.querySelectorAll(".twoslash-popup-container")].some((el) => {
        const r = el.getBoundingClientRect();
        const s = getComputedStyle(el);
        return s.display !== "none" && s.visibility !== "hidden" && r.width > 0 && r.height > 0;
      }),
    );
  const shownAtLoad = await tab.evaluate(
    () =>
      [...document.querySelectorAll(".twoslash-popup-container")].filter((el) => getComputedStyle(el).display !== "none")
        .length,
  );
  await token.scrollIntoViewIfNeeded();
  const before = await shown();
  await token.hover();
  await tab.waitForTimeout(400);
  const mouse = await shown();
  if (shot !== undefined && mouse) await tab.screenshot({ path: shot, clip: await shotBox(tab, token) });
  await tab.mouse.move(0, 0);
  await tab.waitForTimeout(400);
  const focusable = await token.evaluate(
    (el) => el.tabIndex >= 0 || [...el.querySelectorAll("*")].some((c) => c.tabIndex >= 0),
  );
  await tab.keyboard.press("Tab");
  let keyboard = false;
  for (let i = 0; i < 80 && !keyboard; i++) {
    const inToken = await tab.evaluate(() => document.activeElement?.closest(".twoslash-hover") !== null);
    if (inToken) keyboard = await shown();
    await tab.keyboard.press("Tab");
  }
  await tab.evaluate(() => document.activeElement?.blur());
  await tab.mouse.move(0, 0);
  await tab.waitForTimeout(400);
  await token.tap();
  await tab.waitForTimeout(400);
  const touch = await shown();
  const accessible = await token.locator(".twoslash-popup-container").evaluate((el) => ({
    role: el.getAttribute("role"),
    ariaHidden: el.getAttribute("aria-hidden"),
    text: el.textContent.trim().slice(0, 120),
  }));
  return { tokens, shownAtLoad, before, mouse, focusable, keyboard, touch, accessible };
}

/** The token and the open popup, padded, as a screenshot clip. */
async function shotBox(tab, token) {
  const a = await token.boundingBox();
  const b = await tab
    .locator(".twoslash-popup-container")
    .filter({ visible: true })
    .first()
    .boundingBox();
  const x = Math.max(0, Math.min(a.x, b.x) - 16);
  const y = Math.max(0, Math.min(a.y, b.y) - 16);
  return { x, y, width: Math.max(a.x + a.width, b.x + b.width) - x + 16, height: Math.max(a.y + a.height, b.y + b.height) - y + 16 };
}

/** The scripts and stylesheets the page references, by size on disk (the same in every browser). */
async function assets(html) {
  const refs = [...html.matchAll(/(?:src|href)="(\/_astro\/[^"]+\.(js|css))"/g)].map((m) => [m[1], m[2]]);
  const out = { js: { files: 0, bytes: 0, gzip: 0 }, css: { files: 0, bytes: 0, gzip: 0 } };
  for (const [path, kind] of new Map(refs)) {
    const body = await readFile(join(dist, path));
    out[kind].files++;
    out[kind].bytes += body.length;
    out[kind].gzip += gzipSync(body).length;
  }
  return out;
}

// `upgrade-insecure-requests` would send this local http server's subresources to https in WebKit;
// it concerns transport, not inline styles, so the local copies leave it out.
const current = (await currentCsp()).replace(/;\s*upgrade-insecure-requests/, "");
const target = current.replace(/style-src ([^;]*)'unsafe-inline'\s*/, "style-src $1").replace(/\s+;/g, ";");
const html = await readFile(join(dist, page, "index.html"), "utf8");
const shots = process.env.SHOTS;
const report = { page, html: htmlStats(html), assets: await assets(html), csp: { current, target }, browsers: {} };
for (const [name, engine] of [["chromium", chromium], ["firefox", firefox], ["webkit", webkit]]) {
  report.browsers[name] = {};
  for (const [label, csp] of [["current", current], ["target", target]]) {
    for (const theme of ["light", "dark"]) {
      const { server, origin } = await serve(csp);
      const shot = shots === undefined ? undefined : join(shots, `${name}-${label}-${theme}.png`);
      try {
        report.browsers[name][`${label}-${theme}`] = await visit(engine, origin, theme, shot);
      } finally {
        server.close();
      }
    }
  }
}
console.log(JSON.stringify(report, null, 2));
