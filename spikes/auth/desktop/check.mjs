// Runs the Rust desktop client against the Better Auth Worker (../better-auth, port 8787):
// signs a Chromium in with a magic link, starts the client, opens the address it prints in
// that browser (the system browser's part), and checks the session the client reads back.
// Run: cargo build --release, then node check.mjs.

import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { createInterface } from "node:readline";

const { chromium } = createRequire(import.meta.url)("playwright");
const BASE = "http://localhost:8787";
const browser = await chromium.launch();
const page = await browser.newPage();
const email = `desktop-${Date.now()}@example.com`;
await page.goto(BASE);
await page.evaluate((e) => window.api("/api/auth/sign-in/magic-link", { email: e, callbackURL: "/" }), email);
const link = (await (await fetch(`${BASE}/_spike/outbox?email=${encodeURIComponent(email)}`)).json()).url;
await page.goto(link);

const app = spawn("target/release/auth-spike-desktop", ["localhost:8787"]);
const lines = createInterface({ input: app.stdout });
const out = [];
for await (const line of lines) {
  out.push(line);
  if (line.startsWith("http://")) page.goto(line).catch(() => {});
}
const session = JSON.parse(out.find((l) => l.startsWith("SESSION ")).slice(8));
const ok = session?.user?.email === email;
console.log(`${ok ? "ok  " : "FAIL"} Rust client signs in through the browser (${out.find((l) => l.startsWith("ELAPSED_MS"))})`);
await browser.close();
process.exit(ok ? 0 : 1);
