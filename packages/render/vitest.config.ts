import { defineConfig } from "vitest/config";

// The planner's and the error's unit tests, in Node. The contract suite the adapters run is in
// @laminapdf/test-utils. A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
