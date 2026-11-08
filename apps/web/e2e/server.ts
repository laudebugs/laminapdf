// The e2e suite's server: `wrangler dev --local` over the built dist/, on a free port, for the whole
// run. Its address reaches the tests through Vitest's `provide` (inject("origin")).
import { type ChildProcess, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createServer } from "node:net";
import type { TestProject } from "vitest/node";

const APP = new URL("../", import.meta.url);

declare module "vitest" {
  export interface ProvidedContext {
    origin: string;
  }
}

/** A port nothing listens on now. */
function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => (typeof address === "object" && address !== null ? resolve(address.port) : reject()));
    });
  });
}

/** Resolves once `origin` answers, or rejects after `limitMs` or if `child` exits first. */
async function ready(origin: string, child: ChildProcess, limitMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < limitMs) {
    if (child.exitCode !== null) throw new Error(`wrangler dev exited with ${child.exitCode}`);
    try {
      if ((await fetch(origin)).ok) return;
    } catch {
      // Not listening yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`wrangler dev didn't answer on ${origin} within ${limitMs} ms`);
}

export default async function setup(project: TestProject): Promise<() => void> {
  if (!existsSync(new URL("dist/index.html", APP))) {
    throw new Error("apps/web/dist/ is missing: run `pnpm --filter @laminapdf/web build` first");
  }
  const port = await freePort();
  const child = spawn("pnpm", ["exec", "wrangler", "dev", "--local", "--port", String(port), "--ip", "127.0.0.1"], {
    cwd: APP,
    stdio: ["ignore", "inherit", "inherit"],
    env: { ...process.env, WRANGLER_SEND_METRICS: "false" },
  });
  const origin = `http://127.0.0.1:${port}`;
  try {
    await ready(origin, child, 90_000);
  } catch (error) {
    child.kill();
    throw error;
  }
  project.provide("origin", origin);
  return () => {
    child.kill();
  };
}
