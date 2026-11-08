import { defineConfig } from "vitest/config";

// The launcher's platform resolution and the platform packages tools/cli-release generates, in Node.
// A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["test/*.test.ts"] },
});
