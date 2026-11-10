import { defineConfig } from "vitest/config";

// The app's modules in Node, with Tauri's IPC mocked (@tauri-apps/api/mocks). The app in the
// three OS webviews is D2e's WebdriverIO suite. A default export because Vitest reads one.
export default defineConfig({
  test: { environment: "node", include: ["src/**/*.test.ts", "test/**/*.test.ts"] },
});
