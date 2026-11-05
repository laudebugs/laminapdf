/// <reference types="vitest/config" />
// Vitest with the site's own Vite setup (astro.config.mjs), so a test can render .astro components
// with Astro's container API and import files with `?raw` as the pages do.
import { getViteConfig } from "astro/config";

export default getViteConfig({ test: { include: ["test/**/*.test.ts"] } });
