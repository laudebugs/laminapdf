import { defineConfig, mergeConfig } from "vitest/config";
import vite from "./vite.config.ts";

// The Node tests in test/ (coverage, the image pin); e2e/ is Playwright's. A default export because
// Vitest reads one.
export default mergeConfig(vite, defineConfig({ test: { include: ["test/**/*.test.ts"] } }));
