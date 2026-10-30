import { defineConfig } from "vitest/config";

// The workerd run of the B6c matrix: test/workerd/examples.test.ts (in Node) drives Workers
// that wrangler's test harness bundles (the `workerd` export condition) and runs in workerd.
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    environment: "node",
    include: ["test/workerd/*.test.ts"],
    testTimeout: 60_000,
  },
});
