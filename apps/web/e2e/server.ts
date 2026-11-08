// The e2e suite's server: `wrangler dev --local` over the built dist/ (@laminapdf/test-utils'
// wranglerDev, over HTTPS). Its address reaches the tests through Vitest's `provide` (inject("origin")).
import { existsSync } from "node:fs";
import { wranglerDev } from "@laminapdf/test-utils/wrangler-dev";
import type { TestProject } from "vitest/node";

const APP = new URL("../", import.meta.url);

declare module "vitest" {
  export interface ProvidedContext {
    origin: string;
  }
}

export default async function setup(project: TestProject): Promise<() => void> {
  if (!existsSync(new URL("dist/index.html", APP))) {
    throw new Error("apps/web/dist/ is missing: run `pnpm --filter @laminapdf/web build` first");
  }
  const server = await wranglerDev(APP);
  project.provide("origin", server.origin);
  return server.stop;
}
