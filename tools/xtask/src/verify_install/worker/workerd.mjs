// `cargo xtask verify-install workerd`: starts `wrangler dev --local` (the repository's pinned
// wrangler, LAMINA_WRANGLER) in this folder, waits for the Worker to answer, prints its check line,
// then stops wrangler.
import { spawn } from "node:child_process";
import { cwd, env } from "node:process";
import { setTimeout as sleep } from "node:timers/promises";
import { freePort } from "./free-port.mjs";

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
    // A refused connection or a 5xx (wrangler answers 503 while workerd starts) is retried
    // until the deadline; any other answer that isn't 200 fails at once.
    let last = "no answer";
    try {
      const response = await fetch(`http://127.0.0.1:${port}/`);
      if (response.ok) {
        console.log(await response.text());
        break;
      }
      last = `${response.status}: ${await response.text()}`;
      if (response.status < 500) throw new Error(`the Worker answered ${last}`);
    } catch (error) {
      if (error instanceof Error && error.message.startsWith("the Worker answered")) throw error;
      if (error instanceof Error && last === "no answer") last = error.message;
    }
    if (Date.now() > deadline) throw new Error(`the Worker didn't answer within 5 minutes (last: ${last})`);
    await sleep(1000);
  }
} finally {
  wrangler.kill("SIGTERM");
}
