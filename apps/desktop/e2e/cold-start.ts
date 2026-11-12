// The desktop reader's cold start (desktop plan §4: < 300 ms to an empty window that accepts
// input), measured on the e2e build: five launches, each timed by the app itself from the start
// of `run()` to the empty state's Choose a file being on screen (`e2e_interactive`). Reports the
// runs and their median, and writes them to the job summary in CI. It asserts nothing: the gate is
// measured on the reference machine, and D2g brings the app under it.
// Run with `pnpm --filter @laminapdf/desktop cold-start` after `desktop:build:e2e`.
import { spawn } from "node:child_process";
import { appendFileSync } from "node:fs";
import { join } from "node:path";
import { env, platform } from "node:process";
import { fileURLToPath } from "node:url";

const root = join(fileURLToPath(new URL(".", import.meta.url)), "..", "..", "..");
const binary = join(root, "target", "debug", platform === "win32" ? "laminapdf-desktop.exe" : "laminapdf-desktop");
const RUNS = 5;
/** How long a launch may take before the run gives up on it. */
const LIMIT_MS = 30_000;
const TARGET_MS = 300;

/** Launches the app once and resolves with the milliseconds it reported, then stops it. */
function launch(): Promise<number> {
  return new Promise((resolve, reject) => {
    const app = spawn(binary, [], { stdio: ["ignore", "pipe", "inherit"] });
    const timer = setTimeout(() => {
      app.kill();
      reject(new Error(`the app didn't report within ${LIMIT_MS} ms`));
    }, LIMIT_MS);
    let output = "";
    app.stdout.setEncoding("utf8");
    app.stdout.on("data", (chunk: string) => {
      output += chunk;
      const found = /e2e: interactive (\d+) ms/.exec(output);
      if (found === null) return;
      clearTimeout(timer);
      app.once("exit", () => resolve(Number(found[1])));
      app.kill();
    });
    app.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

const runs: number[] = [];
for (let run = 0; run < RUNS; run++) runs.push(await launch());
const median = [...runs].sort((a, b) => a - b)[Math.floor(RUNS / 2)] ?? 0;
const line = `Cold start (e2e build, ${platform}): median ${median} ms over ${RUNS} launches (${runs.join(", ")} ms); target < ${TARGET_MS} ms`;
console.log(line);
if (env.GITHUB_STEP_SUMMARY !== undefined) appendFileSync(env.GITHUB_STEP_SUMMARY, `${line}\n`);
