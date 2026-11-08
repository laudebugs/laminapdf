// Waits until the reader at LAMINA_PDF_SMOKE_URL answers with the app, before the smoke test runs
// (`pnpm smoke` runs this with Node, then Vitest). A Workers custom domain's first deploy creates
// its DNS record and certificate, which can take minutes, and a failed smoke test rolls production
// back, so the test starts once the host serves the page rather than failing on the way.
//
// It runs outside Vitest: a global setup that throws leaves Vitest printing "No test files found"
// and a source-map error instead of the reason.

import { env, exit } from "node:process";

/** How long a deploy may take to answer, and how often to ask. */
const LIMIT_MS = 10 * 60_000;
const EVERY_MS = 10_000;
/** A refusal (401, 403, 407) this many times in a row won't go away by waiting. */
const REFUSALS = 3;

/** What one answer was, and who gave it. */
interface Answer {
  ok: boolean;
  status: number;
  summary: string;
}

/** Prints `message` (a GitHub annotation in Actions) and exits 1. */
function fail(message: string): never {
  console.error(
    env.GITHUB_ACTIONS === "true" ? `::error title=Smoke test::${message.replaceAll("\n", "%0A")}` : message,
  );
  exit(1);
}

/** The headers that tell Cloudflare's answers from a proxy's, and the page's title. */
function summarise(response: Response, body: string): string {
  const parts = [`HTTP ${response.status}`];
  for (const name of ["server", "cf-ray", "cf-mitigated", "via", "location"]) {
    const value = response.headers.get(name);
    if (value !== null) parts.push(`${name}: ${value}`);
  }
  const title = body.match(/<title[^>]*>([^<]{1,120})/i)?.[1]?.trim();
  if (title) parts.push(`title: ${JSON.stringify(title)}`);
  else if (body.trim() !== "") parts.push(`body: ${JSON.stringify(body.trim().slice(0, 120))}`);
  return parts.join(", ");
}

async function ask(origin: string): Promise<Answer> {
  const response = await fetch(origin, { redirect: "manual" });
  const body = await response.text();
  return {
    ok: response.ok && body.includes('<div id="app">'),
    status: response.status,
    summary: summarise(response, body),
  };
}

/** Why a refusal happened: Cloudflare stamps every answer it gives with cf-ray. */
function explainRefusal(origin: string, summary: string): string {
  const runner = env.RUNNER_NAME ? ` on the runner ${env.RUNNER_NAME}` : "";
  const intercepted = ["HTTPS_PROXY", "https_proxy", "NODE_TLS_REJECT_UNAUTHORIZED", "NODE_EXTRA_CA_CERTS"]
    .filter((name) => env[name] !== undefined && env[name] !== "")
    .join(", ");
  const lines = [`${origin} refused this machine${runner}, ${REFUSALS} times in a row (${summary}).`];
  if (summary.includes("cf-ray:")) {
    lines.push(
      "The answer came from Cloudflare: look for Cloudflare Access or a security rule on the host (apps/web/DEPLOY.md, Troubleshooting).",
    );
  } else {
    lines.push(
      "The answer has no cf-ray header, so Cloudflare didn't give it: a proxy or security gateway between this machine and Cloudflare refused the request (apps/web/DEPLOY.md, Troubleshooting).",
    );
  }
  if (intercepted !== "") lines.push(`This machine sets ${intercepted}, which points to such a proxy.`);
  return lines.join("\n");
}

async function main(): Promise<void> {
  const origin = env.LAMINA_PDF_SMOKE_URL;
  if (origin === undefined || origin === "") fail("set LAMINA_PDF_SMOKE_URL to the deployed reader's address");
  const start = Date.now();
  let last = "no answer yet";
  let refused = 0;
  while (Date.now() - start < LIMIT_MS) {
    try {
      const answer = await ask(origin);
      if (answer.ok) {
        console.log(`${origin} serves the reader.`);
        return;
      }
      last = answer.summary;
      refused = [401, 403, 407].includes(answer.status) ? refused + 1 : 0;
      if (refused >= REFUSALS) fail(explainRefusal(origin, last));
    } catch (error) {
      last = String(error);
      refused = 0;
    }
    console.log(`waiting for ${origin}: ${last}`);
    await new Promise((resolve) => setTimeout(resolve, EVERY_MS));
  }
  fail(`${origin} didn't serve the reader within ${LIMIT_MS / 60_000} minutes (last: ${last})`);
}

await main();
