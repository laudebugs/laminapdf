import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

// The app's modules in Node, with Tauri's IPC mocked (@tauri-apps/api/mocks). The app in the
// three OS webviews is D2e's WebdriverIO suite. A default export because Vitest reads one.
export default defineConfig({
  // The viewer's edit history from its source, so these tests need no viewer build.
  resolve: {
    alias: {
      "@laminapdf/viewer/edit-history": fileURLToPath(
        new URL("../../packages/viewer/src/worker/session.ts", import.meta.url),
      ),
    },
  },
  test: { environment: "node", include: ["src/**/*.test.ts", "test/**/*.test.ts"] },
});
