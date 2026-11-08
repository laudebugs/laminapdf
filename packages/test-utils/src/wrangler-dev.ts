// A site's e2e server: `wrangler dev --local` over its built dist/, on a free port, for a whole
// Vitest run, over HTTPS. The sites' policies end in `upgrade-insecure-requests`, which WebKit
// applies even to 127.0.0.1, so a plain-HTTP server would have every script refused there.
// `getLocal` fetches from it, accepting Wrangler's self-signed certificate for those requests
// alone; Node's own checks stay on for everything else. Used by apps/web's and apps/docs' e2e suites.
import { type ChildProcess, spawn } from "node:child_process";
import type { IncomingHttpHeaders } from "node:http";
import { request } from "node:https";
import { createServer } from "node:net";
import { env, kill } from "node:process";
import { fileURLToPath } from "node:url";

/** Node's response headers as a `Headers`, a repeated header kept once per value. */
function toHeaders(raw: IncomingHttpHeaders): Headers {
  const headers = new Headers();
  for (const [name, value] of Object.entries(raw)) {
    for (const v of [value].flat()) if (v !== undefined) headers.append(name, v);
  }
  return headers;
}

/** `GET url` from the local server, not following redirects, as a `Response`. */
export function getLocal(url: URL): Promise<Response> {
  return new Promise((resolve, reject) => {
    const req = request(url, { method: "GET", rejectUnauthorized: false }, (res) => {
      const chunks: Uint8Array<ArrayBuffer>[] = [];
      res.on("data", (chunk: Uint8Array<ArrayBuffer>) => chunks.push(chunk));
      res.on("end", () => {
        const status = res.statusCode ?? 0;
        const body = status === 204 || status === 304 ? null : new Blob(chunks);
        resolve(new Response(body, { status, headers: toHeaders(res.headers) }));
      });
      res.on("error", reject);
    });
    req.on("error", reject);
    req.end();
  });
}

/** Wrangler exited before it answered. */
class Exited extends Error {}

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
    if (child.exitCode !== null) throw new Exited(`wrangler dev exited with ${child.exitCode}`);
    try {
      if ((await getLocal(new URL(origin))).ok) return;
    } catch {
      // Not listening yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`wrangler dev didn't answer on ${origin} within ${limitMs} ms`);
}

/** A running server: its origin and how to stop it. */
export interface WranglerDev {
  readonly origin: string;
  stop(): void;
}

/** How many ports to try: another process can take the free port before Wrangler binds it. */
const ATTEMPTS = 3;

/**
 * Starts `wrangler dev` in `app`, the folder with the site's wrangler config and its own Wrangler,
 * and waits until it answers. A Wrangler that exits before answering (its port was taken between
 * `freePort` and its start) is tried again on a new port.
 */
export async function wranglerDev(app: URL): Promise<WranglerDev> {
  for (let attempt = 1; ; attempt++) {
    try {
      return await start(app, await freePort());
    } catch (error) {
      if (attempt >= ATTEMPTS || !(error instanceof Exited)) throw error;
    }
  }
}

async function start(app: URL, port: number): Promise<WranglerDev> {
  // Wrangler's own binary, in a process group of its own: stopping the group stops the workerd it
  // starts too, which would otherwise outlive the run on a shared runner.
  const wrangler = fileURLToPath(new URL("node_modules/.bin/wrangler", app));
  const child = spawn(
    wrangler,
    ["dev", "--local", "--local-protocol", "https", "--port", String(port), "--ip", "127.0.0.1"],
    {
      cwd: app,
      detached: true,
      stdio: ["ignore", "inherit", "inherit"],
      env: { ...env, WRANGLER_SEND_METRICS: "false" },
    },
  );
  const stop = () => {
    if (child.pid !== undefined && child.exitCode === null) kill(-child.pid, "SIGTERM");
  };
  const origin = `https://127.0.0.1:${port}`;
  try {
    await ready(origin, child, 90_000);
  } catch (error) {
    stop();
    throw error;
  }
  return { origin, stop };
}
