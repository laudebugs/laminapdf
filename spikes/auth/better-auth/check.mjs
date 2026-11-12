// The scripted checks the README's table reports, against `wrangler dev --local` on port 8787
// (local D1). Chromium (Playwright) is the browser; its virtual authenticator stands in for a
// passkey. Run: node check.mjs. Prints one line per check and exits 1 on a failure.

import { createHash, randomBytes } from "node:crypto";
import { createServer } from "node:http";
import { createRequire } from "node:module";

const { chromium } = createRequire(import.meta.url)("playwright");
const BASE = process.env.BASE ?? "http://localhost:8787";
const results = [];
const check = (name, ok, detail = "") => {
  results.push({ name, ok, detail });
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` (${detail})` : ""}`);
};
const b64url = (buf) => buf.toString("base64url");

const browser = await chromium.launch();
const context = await browser.newContext();
const page = await context.newPage();
await page.goto(BASE);
const api = (path, body) => page.evaluate(([p, b]) => window.api(p, b), [path, body]);

// 1. Email magic link: the link signs the browser in; the session cookie is scoped as ADR 0011 asks.
const email = `reader-${Date.now()}@example.com`;
let started = performance.now();
const sent = await api("/api/auth/sign-in/magic-link", { email, callbackURL: "/" });
const link = (await (await fetch(`${BASE}/_spike/outbox?email=${encodeURIComponent(email)}`)).json()).url;
await page.goto(link);
const me = await api("/api/auth/get-session");
check("magic link signs in", sent.status === 200 && me.body?.user?.email === email,
  `${Math.round(performance.now() - started)} ms for request + verify`);
const cookies = await context.cookies();
const sessionCookie = cookies.find((c) => c.name.includes("session_token"));
check("session cookie: HttpOnly, Secure, SameSite=Strict, /api only",
  sessionCookie?.httpOnly === true && sessionCookie.secure && sessionCookie.sameSite === "Strict" && sessionCookie.path === "/api",
  cookies.map((c) => `${c.name} ${c.sameSite} ${c.path}`).join("; "));
const pageCookies = await page.evaluate(() => document.cookie);
check("no cookie readable by the page", pageCookies === "", JSON.stringify(pageCookies));
const reused = await fetch(link, { redirect: "manual" });
check("a magic link works once", !(reused.headers.get("set-cookie") ?? "").includes("session_token"));

// 2. Passkeys with Chromium's virtual authenticator: register while signed in, sign out, sign in.
const cdp = await context.newCDPSession(page);
await cdp.send("WebAuthn.enable");
await cdp.send("WebAuthn.addVirtualAuthenticator", {
  options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true },
});
const registered = await page.evaluate(() => window.registerPasskey());
check("passkey registers", registered.status === 200, JSON.stringify(registered.body).slice(0, 80));
await api("/api/auth/sign-out", {});
check("signed out", (await api("/api/auth/get-session")).body === null);
const passkeyIn = await page.evaluate(() => window.signInWithPasskey());
const afterPasskey = await api("/api/auth/get-session");
check("passkey signs in", passkeyIn.status === 200 && afterPasskey.body?.user?.email === email);

// 3. OAuth state and SameSite: the provider's redirect back is a cross-site navigation, so the
// browser leaves out Strict cookies. Start a GitHub sign-in, then call the callback the way the
// browser would: with the state cookie only when it isn't Strict.
const oauth = await fetch(`${BASE}/api/auth/sign-in/social`, {
  method: "POST",
  headers: { "content-type": "application/json", origin: BASE },
  body: JSON.stringify({ provider: "github", callbackURL: "/" }),
});
const oauthBody = await oauth.json();
const state = new URL(oauthBody.url).searchParams.get("state");
const stateCookie = oauth.headers.getSetCookie().find((c) => /state=/.test(c)) ?? "";
const strict = /samesite=strict/i.test(stateCookie);
const callback = await fetch(`${BASE}/api/auth/callback/github?code=fake&state=${state}`, {
  redirect: "manual",
  headers: strict ? {} : { cookie: stateCookie.split(";")[0] },
});
const where = callback.headers.get("location") ?? "";
check(`OAuth state cookie is ${strict ? "Strict" : "Lax"}`, true, stateCookie.split(";").slice(1).join(";").trim());
check("OAuth callback gets past the state check", !/state_mismatch|please_restart_the_process/.test(where), where);

// 4. Desktop sign-in: a loopback listener, PKCE, the browser's session authorises, the app gets its own token.
const verifier = b64url(randomBytes(32));
const challenge = b64url(createHash("sha256").update(verifier).digest());
const appState = b64url(randomBytes(16));
const got = new Promise((resolve) => {
  const server = createServer((req, res) => {
    const u = new URL(req.url, "http://127.0.0.1");
    res.end("You can close this tab and go back to Lamina PDF.");
    server.close();
    resolve(u.searchParams);
  }).listen(0, "127.0.0.1", () => {
    const port = server.address().port;
    const authorize = `${BASE}/api/desktop/authorize?redirect_uri=${encodeURIComponent(`http://127.0.0.1:${port}/cb`)}&code_challenge=${challenge}&state=${appState}`;
    page.goto(authorize).catch(() => {});
  });
});
const params = await got;
check("loopback gets a code and its state back", params.get("state") === appState && (params.get("code") ?? "").length > 10);
const wrong = await fetch(`${BASE}/api/desktop/token`, { method: "POST", body: JSON.stringify({ code: params.get("code"), code_verifier: b64url(randomBytes(32)) }) });
check("a wrong verifier is refused", wrong.status === 400);
const params2 = await new Promise((resolve) => {
  const server = createServer((req, res) => {
    res.end("ok");
    server.close();
    resolve(new URL(req.url, "http://127.0.0.1").searchParams);
  }).listen(0, "127.0.0.1", () => {
    page.goto(`${BASE}/api/desktop/authorize?redirect_uri=${encodeURIComponent(`http://127.0.0.1:${server.address().port}/cb`)}&code_challenge=${challenge}&state=${appState}`).catch(() => {});
  });
});
const token = await (await fetch(`${BASE}/api/desktop/token`, { method: "POST", body: JSON.stringify({ code: params2.get("code"), code_verifier: verifier }) })).json();
const desktopSession = await (await fetch(`${BASE}/api/auth/get-session`, { headers: { authorization: `Bearer ${token.token}` } })).json();
check("desktop token works as a bearer session", desktopSession?.user?.email === email);
const replay = await fetch(`${BASE}/api/desktop/token`, { method: "POST", body: JSON.stringify({ code: params2.get("code"), code_verifier: verifier }) });
check("a code works once", replay.status === 400);
const evil = await fetch(`${BASE}/api/desktop/authorize?redirect_uri=${encodeURIComponent("https://evil.example/cb")}&code_challenge=${challenge}&state=x`);
check("a non-loopback redirect is refused", evil.status === 400);

// 5. Device authorization (RFC 8628), the other built-in desktop option.
await page.goto(BASE);
const device = await (await fetch(`${BASE}/api/auth/device/code`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ client_id: "lamina-desktop" }) })).json();
await api(`/api/auth/device?user_code=${device.user_code}`);
const approve = await api("/api/auth/device/approve", { userCode: device.user_code });
const deviceToken = await (await fetch(`${BASE}/api/auth/device/token`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ grant_type: "urn:ietf:params:oauth:grant-type:device_code", device_code: device.device_code, client_id: "lamina-desktop" }) })).json();
check("device authorization grant", approve.status === 200 && typeof deviceToken.access_token === "string", `${approve.status} ${JSON.stringify(approve.body)} ${JSON.stringify(deviceToken).slice(0, 80)}`);

// 6. Export and deletion.
const exported = await api("/api/me/export");
check("account export", exported.status === 200 && exported.body.passkeys.length === 1, `${JSON.stringify(exported.body).length} bytes`);
const deleted = await api("/api/auth/delete-user", {});
const gone = await (await fetch(`${BASE}/api/auth/get-session`, { headers: { authorization: `Bearer ${token.token}` } })).json();
check("account deletion ends every session", deleted.status === 200 && gone === null, JSON.stringify(deleted.body));

await browser.close();
process.exit(results.every((r) => r.ok) ? 0 : 1);
