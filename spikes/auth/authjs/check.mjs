// The scripted checks for the Auth.js prototype, against `wrangler dev --local` on port 8788.
// Run: node check.mjs. Same checks as ../better-auth/check.mjs where Auth.js has the feature.

import { createRequire } from "node:module";

const { chromium } = createRequire(import.meta.url)("playwright");
const BASE = process.env.BASE ?? "http://localhost:8788";
const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok, detail });
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` (${detail})` : ""}`);
};

await fetch(`${BASE}/_spike/migrate`, { method: "POST" });
const browser = await chromium.launch();
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(`${BASE}/api/auth/signin`);
const post = (path, form) =>
  page.evaluate(async ([p, f]) => {
    const r = await fetch(p, { method: "POST", body: new URLSearchParams(f), redirect: "manual" });
    return { status: r.status, type: r.type };
  }, [path, form]);
const json = (path) => page.evaluate((p) => fetch(p).then((r) => r.json()), path);

// 1. Email link.
const email = `reader-${Date.now()}@example.com`;
const { csrfToken } = await json("/api/auth/csrf");
await post("/api/auth/signin/email", { email, csrfToken, callbackUrl: `${BASE}/api/auth/session` });
const link = (await (await fetch(`${BASE}/_spike/outbox?email=${encodeURIComponent(email)}`)).json()).url;
await page.goto(link);
const session = await json("/api/auth/session");
check("email link signs in", session?.user?.email === email, link ? "" : "no link sent");
const cookies = await context.cookies();
const token = cookies.find((c) => c.name.includes("session-token"));
check("session cookie: HttpOnly, Secure, SameSite=Strict, /api only",
  token?.httpOnly === true && token.secure && token.sameSite === "Strict" && token.path === "/api",
  cookies.map((c) => `${c.name} ${c.sameSite} ${c.path}`).join("; "));
const again = await fetch(link, { redirect: "manual" });
check("an email link works once", !(again.headers.get("location") ?? "").includes("session") || (again.headers.get("location") ?? "").includes("error"), again.headers.get("location") ?? "");

// 2. Passkey: the provider needs the adapter's authenticator methods.
const options = await page.evaluate(() => fetch("/api/auth/webauthn-options/passkey?action=register").then(async (r) => ({ status: r.status, body: await r.text() })));
check("passkey registration options", options.status === 200, `${options.status}: ${options.body.slice(0, 80)}`);

// 3. OAuth state and SameSite, as in the Better Auth check.
// A server-side request, so the redirect to github.com isn't followed.
const signin = await fetch(`${BASE}/api/auth/signin/github`, {
  method: "POST",
  redirect: "manual",
  headers: {
    "content-type": "application/x-www-form-urlencoded",
    cookie: (await context.cookies()).filter((c) => c.name.includes("csrf")).map((c) => `${c.name}=${c.value}`).join("; "),
  },
  body: new URLSearchParams({ csrfToken: (await json("/api/auth/csrf")).csrfToken }),
});
const providerUrl = signin.headers.get("location") ?? "";
const setCookies = signin.headers.getSetCookie();
const strict = setCookies.some((c) => /pkce|state/.test(c) && /samesite=strict/i.test(c));
const state = new URL(providerUrl, BASE).searchParams.get("state");
const callback = await fetch(`${BASE}/api/auth/callback/github?code=fake${state ? `&state=${state}` : ""}`, {
  redirect: "manual",
  headers: strict ? {} : { cookie: setCookies.map((c) => c.split(";")[0]).join("; ") },
});
console.log(`note OAuth check cookies (${setCookies.map((c) => c.split(";")[0].split("=")[0]).join(", ")}) are ${strict ? "Strict" : "Lax"}`);
// Auth.js sends every callback failure to ?error=Configuration, so the server's log (LOG, the
// file `wrangler dev` writes to) tells a failed state or PKCE check from GitHub refusing the fake code.
await new Promise((r) => setTimeout(r, 500));
const { readFileSync } = await import("node:fs");
const log = process.env.LOG ? readFileSync(process.env.LOG, "utf8").split("GET /api/auth/callback/github").at(-2) ?? "" : "";
check("OAuth callback gets past the state check", process.env.LOG !== undefined && !/InvalidCheck/.test(log),
  /InvalidCheck/.test(log) ? "InvalidCheck: the check cookie didn't come back" : "reached GitHub's token endpoint");

// 4. Deletion.
const deleted = await page.evaluate(() => fetch("/api/me", { method: "DELETE" }).then(async (r) => ({ status: r.status, body: await r.text() })));
const after = await json("/api/auth/session");
check("account deletion ends the session", deleted.status === 200 && after === null, `${deleted.body} → ${JSON.stringify(after)}`);

await browser.close();
process.exit(results.every((r) => r.ok) ? 0 : 1);
