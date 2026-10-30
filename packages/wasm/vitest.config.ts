import { defineConfig } from "vitest/config";

// Node only here; the browser, workerd, Deno and Bun runs are B6c's runtime matrix.
// A default export because Vitest reads one (the named-exports rule is for package code).
export default defineConfig({
  test: {
    environment: "node",
    include: ["test/**/*.test.ts"],
  },
});
