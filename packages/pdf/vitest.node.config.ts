import { defineConfig } from "vitest/config";

// Node (the `node` export condition, against the built dist/); test/vite.test.ts bundles the
// package for browsers with Vite. The other runtimes of the B6c matrix have their own runners:
// vitest.browser.config.ts (Chromium, Firefox, WebKit), vitest.workers.config.ts (workerd),
// test/deno/ and test/bun/; all run test/suite/examples.ts.
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    environment: "node",
    include: ["test/*.test.ts"],
    testTimeout: 60_000,
  },
});
