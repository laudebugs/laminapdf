// Lighthouse over the built reader as production serves it (todo C7c): `wrangler dev --local`
// over dist/ with the deployed headers (@laminapdf/test-utils' wranglerDev, as the e2e suite
// uses), then `lhci autorun` with lighthouserc.json's thresholds. The release lane runs it
// before production deploys; `pnpm --filter @laminapdf/web lighthouse:local` after a build.
import { spawnSync } from "node:child_process";
import process from "node:process";
import { wranglerDev } from "@laminapdf/test-utils/wrangler-dev";

const server = await wranglerDev(new URL("../", import.meta.url));
try {
  // The local server's certificate is the suite's own: Chrome is told to accept it.
  const run = spawnSync(
    "pnpm",
    [
      "exec",
      "lhci",
      "autorun",
      "--config=lighthouserc.json",
      `--collect.url=${server.origin}/`,
      "--collect.settings.chromeFlags=--headless=new --no-sandbox --ignore-certificate-errors",
    ],
    { cwd: new URL("../", import.meta.url), stdio: "inherit" },
  );
  process.exitCode = run.status ?? 1;
} finally {
  await server.stop();
}
