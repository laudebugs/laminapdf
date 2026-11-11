// The e2e suite's server: `wrangler dev --local` over the built dist/ with its _headers
// (@laminapdf/test-utils' wranglerDev, over HTTPS). Its address reaches the tests through Vitest's
// `provide` (inject("origin")).
import { existsSync } from "node:fs";
import { wranglerDev } from "@laminapdf/test-utils/wrangler-dev";
import type { TestProject } from "vitest/node";

const DOCS = new URL("../", import.meta.url);

declare module "vitest" {
  export interface ProvidedContext {
    origin: string;
    /** The server's certificate, which getLocal trusts. */
    ca: string;
  }
}

export default async function setup(project: TestProject): Promise<() => void> {
  if (!existsSync(new URL("dist/run/worker.js", DOCS))) {
    throw new Error(
      "apps/docs/dist/ has no runner: run `cargo xtask wasm`, then `pnpm --filter @laminapdf/docs build`",
    );
  }
  const server = await wranglerDev(DOCS);
  project.provide("origin", server.origin);
  project.provide("ca", server.ca);
  return server.stop;
}
