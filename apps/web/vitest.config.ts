import { defineConfig } from "vitest/config";

// The app's pure modules in Node (src/**/*.test.ts) and the build's own checks (test/). The app in
// browsers is vitest.e2e.config.ts.
// A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["src/**/*.test.ts", "test/**/*.test.ts"] },
});
