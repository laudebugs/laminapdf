import { defineConfig } from "vitest/config";

// The planner's and the error's unit tests, in Node. test/contract.ts is not a test file: the
// adapters' suites import it. A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
