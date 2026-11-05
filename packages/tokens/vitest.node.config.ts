import { defineConfig } from "vitest/config";

// Node: the contrast maths, every declared pair, coverage and the generator's output. The
// high-contrast theme's system colours are measured in browsers: vitest.browser.config.ts
// (`cargo xtask pdf-runtimes chromium|firefox|webkit`). A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
