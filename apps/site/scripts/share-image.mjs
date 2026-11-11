// Writes the website's share image (public/og.png, 1200 × 630, for Open Graph and X cards) and its
// PNG favicons (public/favicon-32.png, public/apple-touch-icon.png) and the reader's app icons (public/icon-*.png) in the Paper look, rendered in
// Playwright's Chromium (paper-render.mjs) from the site's own tokens, fonts, favicon.svg and headline (src/data/copy.ts).
// Run it by hand after changing any of them: `pnpm --filter @laminapdf/site share-image`. The docs
// site serves the same three icon files, copied from public/ by apps/docs/plugins/integrations.mjs.
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { OPENING } from "../src/data/copy.ts";
import { PAPER_LINKS, withPaperRenderer } from "./paper-render.mjs";

const at = (path) => fileURLToPath(new URL(path, import.meta.url));

const share = `<!doctype html><html data-theme="light"><head><meta charset="utf-8">
${PAPER_LINKS}
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

await withPaperRenderer(async (render) => {
  await render(share, 1200, 630, at("../public/og.png"));
  await render(icon(32, 0, "transparent"), 32, 32, at("../public/favicon-32.png"), { transparent: true });
  await render(icon(180, 24, "#F7F4EE"), 180, 180, at("../public/apple-touch-icon.png"), { transparent: true });
  // The reader's installed-app icons (its manifest): the same mark on paper at 192 and 512 px, and a
  // maskable 512 px one whose mark stays inside the safe circle (the middle 80 %) any mask keeps.
  await render(icon(192, 26, "#F7F4EE"), 192, 192, at("../public/icon-192.png"));
  await render(icon(512, 68, "#F7F4EE"), 512, 512, at("../public/icon-512.png"));
  await render(icon(512, 128, "#F7F4EE"), 512, 512, at("../public/icon-maskable-512.png"));
});
