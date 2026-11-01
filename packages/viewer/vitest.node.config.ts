import { defineConfig } from "vitest/config";

// Node: the unit tests next to their modules (src/**/*.test.ts), the type tests and the
// dependency check. The worker, RPC and rendering run in
// browsers: vitest.browser.config.ts (`cargo xtask pdf-runtimes chromium|firefox|webkit`).
// A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts", "src/**/*.test.ts"], testTimeout: 60_000 },
});
