import { defineConfig } from "vitest/config";

// Node: the lint over svg/ and its negative fixtures, the generated module, the design-language
// tables, the gallery's contents and the root entry's imports. `<Icon>` and the gallery's
// accessibility run in browsers: vitest.browser.config.ts (`cargo xtask pdf-runtimes
// chromium|firefox|webkit`). A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
