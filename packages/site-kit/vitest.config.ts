// The Node tests (test/*.test.ts): the build steps and the header's links. The browser tests are
// vitest.browser.config.ts's. A default export because Vitest reads one.
import { defineConfig } from "vitest/config";

export default defineConfig({ test: { include: ["test/*.test.ts"] } });
