import { defineConfig } from "vitest/config";

// Node: the RenderBackend contract over packages/wasm/dist/lamina_render_hayro.wasm (built by
// `cargo xtask wasm`, which runs this). The browser runs are vitest.browser.config.ts.
// A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"], testTimeout: 120_000 },
});
