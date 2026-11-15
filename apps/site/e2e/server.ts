// The e2e suite's server: `wrangler dev --local` over the built dist/ with its _headers
// (@laminapdf/test-utils' wranglerDev, over HTTPS). Its address reaches the tests through Vitest's
// `provide` (inject("origin")).
import { existsSync } from "node:fs";
import { wranglerDev } from "@laminapdf/test-utils/wrangler-dev";
import type { TestProject } from "vitest/node";

const SITE = new URL("../", import.meta.url);

declare module "vitest" {
  export interface ProvidedContext {
    origin: string;
  }
}

export default async function setup(project: TestProject): Promise<() => void> {
  if (!existsSync(new URL("dist/index.html", SITE))) {
    throw new Error("apps/site/dist/ isn't built: run `pnpm --filter @laminapdf/site build`");
  }
  const server = await wranglerDev(SITE);
  project.provide("origin", server.origin);
  return server.stop;
}
