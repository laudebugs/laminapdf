// Renders HTML in the Paper look to PNG, in Playwright's Chromium: the page loads the site's own tokens
// and fonts from a made-up origin that a route answers from disk, so nothing is fetched from the
// network. Used by share-image.mjs (the site's share image and favicons) and story-image.mjs (the
// pictures for the posts in docs/stories/).
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

/** The pages load the tokens and fonts from this origin, which the route below answers from disk. */
const ORIGIN = "http://paper.invalid";
const TOKENS = new URL(import.meta.resolve("@laminapdf/tokens/tokens.css"));
const FONTS = new URL(import.meta.resolve("@laminapdf/fonts/fonts.css"));
const TYPES = { ".css": "text/css", ".woff2": "font/woff2", ".html": "text/html" };

/** The `<link>` lines a page needs for the tokens and the fonts. */
export const PAPER_LINKS = `<link rel="stylesheet" href="/tokens.css">
<link rel="stylesheet" href="/fonts.css">`;

/** The file `pathname` names: the page itself, the tokens, or the fonts' style sheet and files. */
function file(pathname, html) {
  if (pathname === "/") return { body: html, type: TYPES[".html"] };
  const url = pathname === "/tokens.css" ? TOKENS : new URL(`.${pathname}`, FONTS);
  return { path: fileURLToPath(url), type: TYPES[pathname.slice(pathname.lastIndexOf("."))] };
}

/**
 * Starts Chromium, calls `draw` with a `render(html, width, height, out, options)` function, and
 * closes it. `out` is a file path; `options.transparent` keeps a transparent background.
 * `LAMINA_PDF_CHROMIUM` names a Chromium binary when Playwright's own isn't installed.
 */
export async function withPaperRenderer(draw) {
  const executablePath = process.env.LAMINA_PDF_CHROMIUM;
  const browser = await chromium.launch(executablePath ? { executablePath } : {});
  try {
    await draw(async (html, width, height, out, options = {}) => {
      const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
      await page.route(`${ORIGIN}/**`, (route) => {
        const { body, path, type } = file(new URL(route.request().url()).pathname, html);
        return route.fulfill(path === undefined ? { body, contentType: type } : { path, contentType: type });
      });
      await page.goto(`${ORIGIN}/`, { waitUntil: "load" });
      await page.evaluate(() => document.fonts.ready);
      // Text that spills past the edges would be cut off without a word: refuse it instead.
      const size = await page.evaluate(() => [
        document.documentElement.scrollWidth,
        document.documentElement.scrollHeight,
      ]);
      if (size[0] > width || size[1] > height) {
        throw new Error(`${out}: the content is ${size[0]} × ${size[1]}, larger than ${width} × ${height}`);
      }
      // A transparent icon keeps its alpha channel; Chromium paints white under it otherwise.
      await page.screenshot({ path: out, omitBackground: options.transparent === true });
      await page.close();
    });
  } finally {
    await browser.close();
  }
}
