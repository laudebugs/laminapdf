import { defineConfig } from "vitest/config";

// The Worker's modules in Node (test/*.test.ts), and the bundled Worker in workerd through
// Miniflare (test/workerd.test.ts). @cloudflare/vitest-pool-workers would run the first set in
// workerd too, but its 0.23 release needs Vitest 4 and the workspace is on Vitest 5.
// A default export because Vitest reads one.
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "scripts/**/*.test.ts", "test/**/*.test.ts"],
    testTimeout: 30_000,
  },
});
