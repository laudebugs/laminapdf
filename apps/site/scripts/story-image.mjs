// Draws the pictures for a story in docs/stories/ (the lamina-story skill): every visuals/*.html in
// the story's folder becomes the PNG beside it, at the size its <meta name="lamina-size"
// content="1080x1350"> gives, in the Paper look (paper-render.mjs: the site's tokens and fonts, no
// network). Run it from the repository root:
//   pnpm --filter @laminapdf/site story-image docs/stories/<story>
import { readdir, readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { withPaperRenderer } from "./paper-render.mjs";

const SIZE = /<meta\s+name="lamina-size"\s+content="(\d+)x(\d+)"/;
const HEADLINE = /<h1[^>]*>([\s\S]*?)<\/h1>/;
/** A headline is read at a glance: the skill allows ten words. */
const MAX_HEADLINE_WORDS = 10;

const story = process.argv[2];
if (story === undefined) {
  console.error("usage: pnpm --filter @laminapdf/site story-image docs/stories/<story>");
  process.exit(2);
}
// pnpm runs the script in apps/site; INIT_CWD is where it was called from.
const folder = resolve(process.env.INIT_CWD ?? process.cwd(), story, "visuals");
const pages = await readdir(folder).then(
  (names) => names.filter((name) => name.endsWith(".html")).sort(),
  // Only a missing folder means "no images yet"; any other failure is reported as itself.
  (error) => {
    if (error.code === "ENOENT") return [];
    throw error;
  },
);
if (pages.length === 0) {
  console.error(`no visuals/*.html in ${folder}`);
  process.exit(1);
}

/** The cards' one style sheet, which every story page links as /story-card.css. */
const files = { "/story-card.css": new URL("story-card.css", import.meta.url) };

await withPaperRenderer(
  async (render) => {
    for (const name of pages) {
      const html = await readFile(resolve(folder, name), "utf8");
      const size = SIZE.exec(html);
      if (size === null) throw new Error(`${name}: no <meta name="lamina-size" content="WIDTHxHEIGHT">`);
      const words = (HEADLINE.exec(html)?.[1] ?? "")
        .replace(/<[^>]*>|&nbsp;/g, " ")
        .trim()
        .split(/\s+/);
      if (words.length > MAX_HEADLINE_WORDS) {
        throw new Error(`${name}: the headline has ${words.length} words; ${MAX_HEADLINE_WORDS} at most`);
      }
      const [width, height] = [Number(size[1]), Number(size[2])];
      const out = resolve(folder, name.replace(/\.html$/, ".png"));
      await render(html, width, height, out);
      console.log(`${out} (${width} × ${height})`);
    }
  },
  { files },
);
