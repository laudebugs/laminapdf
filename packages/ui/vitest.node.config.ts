import { defineConfig } from "vitest/config";

// Node: the rules the components' sources must keep (tokens only in CSS, no store, viewer or
// command imports, every design-language section mapped to a component). The components
// themselves run in browsers: vitest.browser.config.ts (`cargo xtask pdf-runtimes
// chromium|firefox|webkit`). A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
