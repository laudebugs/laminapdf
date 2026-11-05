import { defineConfig } from "vitest/config";

// Node: the lock against the files, fonts.css, the size budgets, glyph coverage and the font-CDN
// guard. Loading the fonts in a page is vitest.browser.config.ts's job (`cargo xtask pdf-runtimes
// chromium|firefox|webkit`). A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
