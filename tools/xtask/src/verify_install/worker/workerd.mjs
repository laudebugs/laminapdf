// `cargo xtask verify-install workerd`: starts `wrangler dev --local` (the repository's pinned
// wrangler, LAMINA_WRANGLER) in this folder, waits for the Worker to answer, prints its check line,
// then stops wrangler.
import { spawn } from "node:child_process";
import { createServer } from "node:net";
import { cwd, env } from "node:process";
import { setTimeout as sleep } from "node:timers/promises";

/** A free TCP port on 127.0.0.1. */
const freePort = () =>
  new Promise((resolve, reject) => {
    const server = createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });

const port = await freePort();
const wrangler = spawn(
  env.LAMINA_WRANGLER ?? "wrangler",
  ["dev", "--local", "--ip", "127.0.0.1", "--port", String(port), "--log-level", "warn"],
  {
    cwd: cwd(),
    env: { ...env, CI: "true", WRANGLER_SEND_METRICS: "false" },
    stdio: ["ignore", "inherit", "inherit"],
  },
);
let exited = false;
wrangler.on("exit", () => {
  exited = true;
});
try {
  // A cold start bundles the Worker and starts workerd; a loaded machine needs minutes.
  const deadline = Date.now() + 300_000;
  for (;;) {
    if (exited) throw new Error("wrangler dev exited before the Worker answered");
    try {
      const response = await fetch(`http://127.0.0.1:${port}/`);
      if (response.ok) {
        const version = response.headers.get("server") ?? "";
        console.log(`LAMINA_VERIFY_VERSION workerd ${version}`.trim());
        console.log(await response.text());
        break;
      }
      throw new Error(`the Worker answered ${response.status}: ${await response.text()}`);
    } catch (error) {
      if (error instanceof Error && error.message.startsWith("the Worker answered")) throw error;
      if (Date.now() > deadline) throw new Error("the Worker didn't answer within 5 minutes");
      await sleep(1000);
    }
  }
} finally {
  wrangler.kill("SIGTERM");
}
