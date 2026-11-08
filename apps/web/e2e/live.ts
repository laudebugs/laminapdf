// The smoke test's setup: waits until the reader at LAMINA_PDF_SMOKE_URL answers with the app.
// A Workers custom domain's first deploy creates its DNS record and certificate, which can take
// minutes, and a failed smoke test rolls production back, so the test starts once the host
// serves the page rather than failing on the way.

import { env } from "node:process";

/** How long a deploy may take to answer, and how often to ask. */
const LIMIT_MS = 10 * 60_000;
const EVERY_MS = 10_000;

/** Fails the run with `message`, printed first: Vitest loses a global setup's error message. */
function fail(message: string): never {
  console.error(message);
  throw new Error(message);
}

export default async function setup(): Promise<void> {
  const origin = env.LAMINA_PDF_SMOKE_URL;
  if (origin === undefined || origin === "") fail("set LAMINA_PDF_SMOKE_URL to the deployed reader's address");
  const start = Date.now();
  let last = "no answer yet";
  while (Date.now() - start < LIMIT_MS) {
    try {
      const response = await fetch(origin, { redirect: "manual" });
      if (response.ok && (await response.text()).includes('<div id="app">')) return;
      last = `HTTP ${response.status}`;
    } catch (error) {
      last = String(error);
    }
    await new Promise((resolve) => setTimeout(resolve, EVERY_MS));
  }
  fail(`${origin} didn't serve the reader within ${LIMIT_MS / 60_000} minutes (last: ${last})`);
}
