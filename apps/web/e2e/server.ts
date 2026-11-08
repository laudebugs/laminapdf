// The e2e suite's server: `wrangler dev --local` over the built dist/, on a free port, for the whole
// run, over HTTPS (see local-tls.ts for why). Its address reaches the tests through Vitest's `provide` (inject("origin")).
import { type ChildProcess, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { createServer } from "node:net";
import { fileURLToPath } from "node:url";
import type { TestProject } from "vitest/node";
import { getLocal } from "./local-tls.ts";

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
      if ((await getLocal(new URL(origin))).ok) return;
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
  // Wrangler's own binary, in a process group of its own: stopping the group stops the workerd it
  // starts too, which would otherwise outlive the run on a shared runner.
  const wrangler = fileURLToPath(new URL("node_modules/.bin/wrangler", APP));
  const child = spawn(
    wrangler,
    ["dev", "--local", "--local-protocol", "https", "--port", String(port), "--ip", "127.0.0.1"],
    {
      cwd: APP,
      detached: true,
      stdio: ["ignore", "inherit", "inherit"],
      env: { ...process.env, WRANGLER_SEND_METRICS: "false" },
    },
  );
  const stop = () => {
    if (child.pid !== undefined && child.exitCode === null) process.kill(-child.pid, "SIGTERM");
  };
  const origin = `https://127.0.0.1:${port}`;
  try {
    await ready(origin, child, 90_000);
  } catch (error) {
    stop();
    throw error;
  }
  project.provide("origin", origin);
  return stop;
}
