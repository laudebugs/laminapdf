// Renders HTML in the Paper look to PNG, in Playwright's Chromium: the page loads the site's own tokens
// and fonts from a made-up origin that a route answers from disk, so nothing is fetched from the
// network: every other request is aborted. Used by share-image.mjs (the site's share image and favicons) and story-image.mjs (the
// pictures for the posts in the stories folder, ~/git/lamina-stories).
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

/** The pages load the tokens and fonts from this origin, which the route below answers from disk. */
const ORIGIN = "http://paper.invalid";
const TOKENS = new URL(import.meta.resolve("@laminapdf/tokens/tokens.css"));
const FONTS = new URL(import.meta.resolve("@laminapdf/fonts/fonts.css"));
/** The fonts' folder: a font file must resolve inside it. */
const FONTS_DIR = new URL(".", FONTS);
const TYPES = { ".css": "text/css", ".woff2": "font/woff2", ".html": "text/html" };

/** The `<link>` lines a page needs for the tokens and the fonts. */
export const PAPER_LINKS = `<link rel="stylesheet" href="/tokens.css">
<link rel="stylesheet" href="/fonts.css">`;

/**
 * What `pathname` names: the page itself, one of `files`, the tokens, or a file inside the fonts'
 * folder (its style sheet and font files); `undefined` for anything else. A page's HTML may come
 * from a story, so a path that would leave the fonts' folder (`/../…`) is refused, never read.
 */
function file(pathname, html, files) {
  if (pathname === "/") return { body: html, type: TYPES[".html"] };
  // No file this renderer serves needs escapes or backslashes; refusing them keeps every path plain.
  if (/[%\\]/.test(pathname)) return undefined;
  const type = TYPES[pathname.slice(pathname.lastIndexOf("."))];
  if (type === undefined) return undefined;
  if (Object.hasOwn(files, pathname)) return { path: fileURLToPath(files[pathname]), type };
  if (pathname === "/tokens.css") return { path: fileURLToPath(TOKENS), type };
  const url = new URL(`.${pathname}`, FONTS);
  return url.href.startsWith(FONTS_DIR.href) ? { path: fileURLToPath(url), type } : undefined;
}

/**
 * Starts Chromium, calls `draw` with a `render(html, width, height, out, options)` function, and
 * closes it. `out` is a file path; `options.transparent` keeps a transparent background. `files`
 * maps more paths the pages may load (`/story-card.css`) to file URLs.
 * `LAMINA_PDF_CHROMIUM` names a Chromium binary when Playwright's own isn't installed.
 *
 * A render fails when the content is larger than the image, or than a `.sheet` (the card the
 * page is drawn on): text cut off without a word would be worse than an error.
 */
export async function withPaperRenderer(draw, { files = {} } = {}) {
  const executablePath = process.env.LAMINA_PDF_CHROMIUM;
  const browser = await chromium.launch(executablePath ? { executablePath } : {});
  try {
    await draw(async (html, width, height, out, options = {}) => {
      const page = await browser.newPage({ viewport: { width, height }, deviceScaleFactor: 1 });
      // Deny by default: only the made-up origin is answered, from the allowed files; every
      // other request (another host, a refused path) is aborted or answered 404.
      await page.route("**/*", (route) => {
        const url = new URL(route.request().url());
        if (url.origin !== ORIGIN) return route.abort();
        const found = file(url.pathname, html, files);
        if (found === undefined) return route.fulfill({ status: 404 });
        const { body, path, type } = found;
        return route.fulfill(path === undefined ? { body, contentType: type } : { path, contentType: type });
      });
      await page.goto(`${ORIGIN}/`, { waitUntil: "load" });
      await page.evaluate(() => document.fonts.ready);
      // Any element's box past the image's edges (margins aside), or a card whose scroll size
      // (which counts content that overflows it, even where it is clipped) exceeds its own.
      const spills = await page.evaluate(
        ([w, h]) => {
          const name = (el) => el.className?.baseVal ?? el.className ?? el.tagName;
          const outside = [...document.body.querySelectorAll("*")].filter((el) => {
            const r = el.getBoundingClientRect();
            return r.width > 0 && (r.left < -1 || r.top < -1 || r.right > w + 1 || r.bottom > h + 1);
          });
          const cards = [...document.querySelectorAll(".sheet")].filter(
            (box) => box.scrollWidth > box.clientWidth + 1 || box.scrollHeight > box.clientHeight + 1,
          );
          return [...outside, ...cards].map((el) => name(el) || el.tagName.toLowerCase());
        },
        [width, height],
      );
      if (spills.length > 0) {
        throw new Error(`${out}: the content spills past its box (${spills.join(", ")}); shorten it`);
      }
      // A transparent icon keeps its alpha channel; Chromium paints white under it otherwise.
      await page.screenshot({ path: out, omitBackground: options.transparent === true });
      await page.close();
    });
  } finally {
    await browser.close();
  }
}
