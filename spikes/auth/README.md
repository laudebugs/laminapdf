# `auth` spike (H1): Better Auth on D1 fits Lamina PDF's hosted service best

Lamina PDF's hosted service needs a login stack before any round ships sign-in ([ADR 0011 §6.5](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#6-the-hosted-service-and-later-server-features)). That stack has to keep the ADR's privacy rules, run on Workers with D1, let forks deploy without a vendor account, and sign in the desktop app through the system browser. This spike built three prototypes and measured each against the [H1 criteria](../../docs/todos/ongoing/H1-auth-spike.md): Better Auth and Auth.js, each on a Worker with D1, and Clerk as the hosted vendor. **Recommendation: Better Auth 1.7 on a Worker with D1.** It needs a Lax OAuth state cookie and a 90-line desktop sign-in with an approval page, and it starts with email magic links and passkeys.

Run on 2026-11-12, on Linux x64, against `wrangler dev --local` 4.157.0 (workerd with local D1) and Playwright's Chromium. Nothing here touched Cloudflare, GitHub or Clerk accounts: the checks stop at the edge of what a local run can prove, and the README says where.

## Results

| Criterion | Better Auth 1.7.14 + D1 | Auth.js (`@auth/core` 0.41.3) + `@auth/d1-adapter` | Clerk (`@clerk/backend` 3.26.4) |
| --- | --- | --- | --- |
| Workers and D1 | Runs; D1 built into its Kysely adapter; migrations from the library (`getMigrations`) | Runs; D1 adapter with its own migrations | The SDK runs on a Worker; users live at Clerk, not in D1 |
| Email magic link | Plugin; one-time links ✔ | Email provider type ✔, but the generic `Email` provider is deprecated and the documented one needs SMTP (Nodemailer), which Workers can't open | Built in (hosted) |
| Passkeys | Plugin; register and sign in pass in Chromium's virtual authenticator ✔ | **Fails**: every request errors with `MissingAdapterMethods` (the D1 adapter lacks the authenticator methods); experimental upstream | Built in (hosted) |
| OAuth (GitHub) | Config ✔; callback reaches GitHub's token endpoint with a Lax state cookie | Same ✔ with Lax check cookies | Built in (hosted) |
| Session model | Session rows in D1, read on every check; an `HttpOnly; Secure; SameSite=Strict` cookie on `/api`; bearer tokens for the desktop app | Session rows in D1, the same cookie ✔ | A JWT in a **non-HttpOnly** `__session` cookie on the app's domain, refreshed from Clerk's own domain |
| ADR 0011 §4 and §6.5 (CSP, cookies) | Meets both: first-party only | Meets both | **Breaks both**: `__session` isn't HttpOnly; Clerk's CSP needs `style-src 'unsafe-inline'` and scripts and frames from `challenges.cloudflare.com` and `*.protect.clerk.com` |
| Desktop sign-in | Loopback + PKCE with an approval page, our own codes and its bearer plugin (`src/desktop.js`, 90 lines) ✔; device authorization grant (RFC 8628) built in ✔ | Nothing built in: the same endpoints would be written against the adapter | Not prototyped; Clerk documents an Electron SDK, and a Tauri app would hand off to Clerk's hosted pages |
| Account deletion and export | `delete-user` route ends every session, in another browser too ✔; export written over D1 (one route, 15 lines) ✔ | Deletion written over the adapter ✔; no export | Dashboard export; deletion through the backend API |
| Licence | MIT | ISC | MIT SDK; proprietary service |
| Cost per user | D1 only: a sign-in writes a few rows, a request reads one session row or none (cache cookie). Workers Paid includes 25 billion rows read and 50 million written a month | Same as Better Auth | Free to 50,000 monthly retained users, then Pro at $20–25 a month plus $0.02 per retained user |
| Data residency | D1's EU jurisdiction, set when the database is created | Same | Not checked: Clerk's public pricing and docs pages name no region |
| Lock-in | Tables in our D1; the code is ours to fork | Same | Users and sessions at Clerk; leaving means a migration |
| Maintenance | Active: releases every few days (1.7.22 on 2026-11-12) | **Winding down**: "Auth.js is now part of Better Auth. We recommend new projects to start with Better Auth" (its README); last release 2026-07-20 | Active |
| Worker bundle (gzip) | 465 KiB | 146 KiB | 14 KiB (verification only) |
| Scripted checks | 19 of 19 pass | 5 of 6 pass (passkeys fail) | 3 of 3 pass |

## What the checks found

### SameSite=Strict breaks OAuth unless the state cookie is Lax

ADR 0011 §6.5 asks for `SameSite=Strict` session cookies, and both libraries can set it. But an OAuth provider sends the browser back to `/api/auth/callback/github` from its own site. Browsers leave Strict cookies out of that cross-site navigation, so the library can't match the state it stored. With every cookie Strict, Better Auth redirects to `?error=state_mismatch` and Auth.js fails with `InvalidCheck` (run with `--var STRICT_STATE:1`). A Lax state cookie, which holds no session and lives 5 minutes in Better Auth, fixes both while the session cookie stays Strict. ADR 0011's current wording forbids that fix, so the amendment below allows it for the state cookie alone.

### Better Auth covers every criterion with its own plugins

Better Auth passed all 19 checks in `better-auth/check.mjs`:
- **Email:** a magic link signs in once and fails the second time.
- **Cookies:** the session cookie is `HttpOnly`, `Secure`, `Strict` and scoped to `/api`, and a script on a page under `/api` reads no cookie.
- **Passkeys:** a passkey registers, then signs in after sign-out.
- **OAuth:** the callback passes the state check and reaches GitHub, which refuses the spike's fake code.
- **Desktop:** the user approves before a code is sent; the flow refuses a wrong verifier, a reused code, a non-loopback redirect and an approval posted from another site; the device grant works too.
- **Export and deletion:** the export lists the passkey and the device grant, and deletion ends the desktop app's session and a second browser's at once.

Two settings matter. Better Auth's cookie cache (`session.cookieCache`) saves a D1 read per request, but a cached session keeps working for up to its `maxAge` after the account is deleted, so the prototype leaves it off. Its telemetry (`@better-auth/telemetry`) is off unless the options or `BETTER_AUTH_TELEMETRY` turn it on; the prototype sets `telemetry: { enabled: false }` all the same.

### The desktop app signs in through the system browser after one click

The desktop flow follows RFC 8252. The app's Rust main process listens on `127.0.0.1` on a free port and opens `/api/desktop/authorize` in the system browser with a PKCE challenge. The browser's own session shows "Sign in Lamina PDF on this computer?", and the user's click sends a one-time code to the loopback port. The app trades the code and its verifier for a bearer session of its own, 152 ms after it opened the page in the local run. `desktop/` is that client in Rust (std plus `sha2`, `base64` and `getrandom`). It skips connections that never send `GET /cb`, as a browser's speculative socket does. `desktop/check.mjs` drives it end to end against the Better Auth Worker.

The approval page is the defence against another program on the same computer. Any local program can open the authorize page with a loopback port and a challenge of its own, so without the click it would get a session silently. A public client can't prove which app it is (RFC 8252 §8.6), so the user's click is the check. The codes are the Worker's own random values, not Better Auth's one-time tokens, because its `/one-time-token/verify` route would trade one for the browser's session without a verifier.

In Lamina PDF, the main process would open the address through `tauri-plugin-opener` from Rust, as ADR 0008 §5 already allows, and keep the token in the OS keychain. The webview gets no new permission, and no custom URL scheme has to be registered on three OSes, which a deep-link flow would need (`tauri-plugin-deep-link`). The device authorization grant is the fallback for a machine with no usable browser. This spike didn't build the Tauri window itself. The whole flow runs in the main process, so the app adds only the opener call and the keychain to the Rust client above.

### Auth.js works but is the wrong start for a new service

Auth.js signed in by email link, set the same Strict cookie, passed the OAuth state check and deleted an account through its adapter. Passkeys stopped it: with the passkey provider on, every request fails with `MissingAdapterMethods`, because the D1 adapter has no authenticator table. The project's own README now sends new projects to Better Auth, and its last release is four months old. Choosing it would mean writing the passkey storage, the deletion and export routes and the desktop flow ourselves, on a library its maintainers are winding down.

### Clerk is the least work, but it breaks the privacy rules

Clerk's backend SDK verified a Clerk-shaped session token on a Worker with only the instance's public key: it refused an expired token and one issued for another origin. The spike signed its own tokens, since it had no Clerk account. The rest comes from Clerk's own documentation, and it rules Clerk out. Its `__session` cookie is readable by JavaScript and set `SameSite=Lax`, which ADR 0011 §6.5 rules out. Its pages need `style-src 'unsafe-inline'` and third-party scripts and frames, which ADR 0011 §4 rules out. A fork would also need its own Clerk account. WorkOS (free to a million monthly active users) has the same hosted shape and wasn't prototyped.

## Recommendation

Better Auth keeps every account in Lamina PDF's own D1 database, behind first-party cookies, with no vendor between a reader and their account. Adopting it through the amendment above lets H2 build accounts on checked ground: the privacy rules stay as they are, with one documented exception for the OAuth state cookie.

## Questions for the owner

The stack works for any answer to these, but the first round of accounts (H2) needs them:

1. **Sign-in methods:** email magic link and passkeys first, with GitHub and Google added later (the recommendation), or passkeys only?
2. **Email sending:** magic links need an email provider (Resend or Postmark, for example), a new vendor with its own privacy note.
3. **Data residency:** create the D1 database in the EU jurisdiction? It can only be set at creation.
4. **Paid plans:** the session can carry an entitlements claim either way, so pricing (U5a) doesn't block the choice.

## Draft amendment to ADR 0011 §6.5

> **Accounts (amended after the [`auth` spike](../../spikes/auth/README.md), H1):**
> - The login stack is **Better Auth** (MIT) on a Worker under `/api`, with users, sessions and passkeys in D1. Its telemetry stays off.
> - Session cookies exist only for signed-in users, only on `/api` routes, as first-party `HttpOnly; Secure; SameSite=Strict` cookies. The OAuth state cookie alone is `SameSite=Lax` and lives at most 10 minutes, because a provider's redirect back is cross-site. Anonymous use stays cookie-free (§3.4).
> - The desktop app signs in through the system browser: a loopback redirect with PKCE (RFC 8252, RFC 7636), opened from Rust, that sends a code only after the user approves on the page, with the session token kept in the OS keychain. The device authorization grant (RFC 8628) is the fallback.
> - Sessions are read from D1 on every check (no cookie cache), so account deletion ends every session at once; the account's rows go within 24 hours (§6.3). A signed-in user can export the rows D1 keeps for the account as one JSON file.
> - Sign-in flows must work without COOP/COEP on the main app (ADR 0009 §5).

## Running it

Each prototype is its own pnpm project outside the workspace, with versions pinned exactly. Playwright comes from the machine (`NODE_PATH="$(npm root -g)"`).

```bash
cd spikes/auth/better-auth && pnpm install --ignore-workspace
```

```bash
printf 'BETTER_AUTH_SECRET=%s\nGITHUB_CLIENT_SECRET=x\n' "$(openssl rand -hex 32)" > .dev.vars
```

Start the Worker with `pnpm dev`, then, in a second terminal:

```bash
curl -X POST localhost:8787/_spike/migrate && NODE_PATH="$(npm root -g)" node check.mjs
```

`authjs/` runs the same way on port 8788 (`AUTH_SECRET` in `.dev.vars`; `LOG=<wrangler's output file> node check.mjs` so the check can tell a state failure from GitHub refusing the fake code). For `clerk/`, `node check.mjs > .dev.vars`, then `pnpm dev` and `node check.mjs --run`. For `desktop/`, `cargo build --release`, then `node check.mjs` with the Better Auth Worker running. Add `--var STRICT_STATE:1` to `pnpm dev` to see OAuth fail with a Strict state cookie, and `--var PASSKEY:1` in `authjs/` to see `MissingAdapterMethods`. `pnpm size` gives the bundle sizes in the table.

