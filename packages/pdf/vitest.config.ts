import { defineConfig } from "vitest/config";

// Node here (the `node` export condition, against the built dist/); test/vite.test.ts bundles the
// package for browsers with Vite. The browser, workerd, Deno and Bun runs are B6c's matrix.
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
    testTimeout: 60_000,
  },
});
