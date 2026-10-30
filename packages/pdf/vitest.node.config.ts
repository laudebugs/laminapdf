import { defineConfig } from "vitest/config";

// Node (the `node` export condition, against the built dist/), as two projects:
// - `node`: test/*.test.ts (test/vite.test.ts bundles the package for browsers with Vite); what
//   `cargo xtask wasm` runs;
// - `measure`: test/node/measure.test.ts, the report's hash and the performance smoke, alone in a
//   fresh module instance; `cargo xtask pdf-runtimes node` runs it first, then `node`.
// The other runtimes of the B6c matrix have their own runners: vitest.browser.config.ts
// (Chromium, Firefox, WebKit), vitest.workers.config.ts (workerd), test/deno/ and test/bun/; all
// run test/suite/examples.ts.
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    environment: "node",
    testTimeout: 60_000,
    projects: [
      { extends: true, test: { name: "node", include: ["test/*.test.ts"] } },
      { extends: true, test: { name: "measure", include: ["test/node/measure.test.ts"] } },
    ],
  },
});
