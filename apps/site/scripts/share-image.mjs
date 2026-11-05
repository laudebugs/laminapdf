// Writes the website's share image (public/og.png, 1200 × 630, for Open Graph and X cards) and its
// PNG favicons (public/favicon-32.png, public/apple-touch-icon.png) in the Paper look, rendered in
// Playwright's Chromium from the site's own tokens, fonts, favicon.svg and headline (src/data/copy.ts).
// Run it by hand after changing any of them: `pnpm --filter @laminapdf/site share-image`.
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { OPENING } from "../src/data/copy.ts";

const at = (path) => new URL(path, import.meta.url);
/** The pages load the tokens and fonts from this origin, which the route below answers from disk. */
const ORIGIN = "http://share.invalid";
const TOKENS = new URL(import.meta.resolve("@laminapdf/tokens/tokens.css"));
const FONTS = new URL(import.meta.resolve("@laminapdf/fonts/fonts.css"));
const TYPES = { ".css": "text/css", ".woff2": "font/woff2", ".html": "text/html" };

const share = `<!doctype html><html data-theme="light"><head><meta charset="utf-8">
<link rel="stylesheet" href="/tokens.css">
<link rel="stylesheet" href="/fonts.css">
<style>
  html, body { margin: 0; width: 1200px; height: 630px; }
  body { background: var(--paper-ground); color: var(--paper-text); font-family: var(--paper-font-sans);
    display: flex; align-items: center; justify-content: center; }
  .sheet { box-sizing: border-box; width: 1064px; height: 510px; padding: 56px 64px; background: var(--paper-surface);
    border-radius: 4px; box-shadow: var(--paper-shadow-page); display: flex; flex-direction: column; gap: 28px; }
  .wordmark { font: 700 34px/1 var(--paper-font-sans); }
  h1 { margin: 0; font: 400 58px/1.12 var(--paper-font-serif); letter-spacing: -0.005em; flex: 1; }
  .foot { display: flex; justify-content: space-between; color: var(--paper-muted); font-size: 24px; }
  .accent { width: 72px; height: 6px; background: var(--paper-accent); border-radius: 3px; }
</style></head><body><div class="sheet">
  <div class="wordmark">Lamina PDF</div><div class="accent"></div>
  <h1>${OPENING.headline}</h1>
  <div class="foot"><span>laminapdf.com</span><span>Rust · TypeScript · the roadmap</span></div>
</div></body></html>`;

const svg = await readFile(at("../public/favicon.svg"), "utf8");
const icon = (size, padding, background) => `<!doctype html><html><head><style>
  html, body { margin: 0; width: ${size}px; height: ${size}px; background: ${background}; }
  svg { display: block; width: ${size - 2 * padding}px; height: ${size - 2 * padding}px; margin: ${padding}px; }
</style></head><body>${svg}</body></html>`;

/** The file `pathname` names: the page itself, the tokens, or the fonts' style sheet and files. */
function file(pathname, html) {
  if (pathname === "/") return { body: html, type: TYPES[".html"] };
  const url = pathname === "/tokens.css" ? TOKENS : new URL(`.${pathname}`, FONTS);
  return { path: fileURLToPath(url), type: TYPES[pathname.slice(pathname.lastIndexOf("."))] };
}

const browser = await chromium.launch();
try {
  const render = async (html, width, height, out) => {
    const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
    await page.route(`${ORIGIN}/**`, (route) => {
      const { body, path, type } = file(new URL(route.request().url()).pathname, html);
      return route.fulfill(path === undefined ? { body, contentType: type } : { path, contentType: type });
    });
    await page.goto(`${ORIGIN}/`, { waitUntil: "load" });
    await page.evaluate(() => document.fonts.ready);
    await page.screenshot({ path: fileURLToPath(at(out)), omitBackground: false });
    await page.close();
  };
  await render(share, 1200, 630, "../public/og.png");
  await render(icon(32, 0, "transparent"), 32, 32, "../public/favicon-32.png");
  await render(icon(180, 24, "#F7F4EE"), 180, 180, "../public/apple-touch-icon.png");
} finally {
  await browser.close();
}
