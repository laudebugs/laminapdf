// Better Auth 1.7.14 on a Worker with D1 (H1 prototype a). Throwaway spike code: the README
// says what each route proves. Everything the hosted service would need from a login stack is
// under /api, as ADR 0011 §6.5 asks: session cookies only there, HttpOnly, Secure, SameSite=Strict.

import { passkey } from "@better-auth/passkey";
import { betterAuth } from "better-auth";
import { getMigrations } from "better-auth/db/migration";
import { bearer } from "better-auth/plugins/bearer";
import { deviceAuthorization } from "better-auth/plugins/device-authorization";
import { magicLink } from "better-auth/plugins/magic-link";
import { desktopRoutes } from "./desktop.js";
import { PAGE } from "./page.js";

/** The last magic link per address: the spike's stand-in for an email provider. */
const outbox = new Map();

/** Per-request options: a Worker sees its bindings only inside `fetch`. */
export function options(env) {
  return {
    appName: "Lamina PDF (auth spike)",
    baseURL: env.BASE_URL,
    basePath: "/api/auth",
    secret: env.BETTER_AUTH_SECRET,
    database: env.DB,
    telemetry: { enabled: false },
    // Sessions live in D1 and every check reads them there. Better Auth's cookie cache would save
    // that read, but a cached session outlives its deletion by up to its maxAge (README).
    session: { expiresIn: 60 * 60 * 24 * 30 },
    user: { deleteUser: { enabled: true } },
    socialProviders: {
      github: { clientId: env.GITHUB_CLIENT_ID, clientSecret: env.GITHUB_CLIENT_SECRET },
    },
    advanced: {
      cookiePrefix: "lamina",
      useSecureCookies: true,
      defaultCookieAttributes: { httpOnly: true, secure: true, sameSite: "strict", path: "/api" },
      // The OAuth state cookie has to come back on the provider's cross-site redirect, which a
      // Strict cookie doesn't (README: "SameSite=Strict and OAuth"). Lax for this one only, unless
      // the STRICT_STATE variable asks to show the failure.
      cookies:
        env.STRICT_STATE === "1" ? {} : { state: { attributes: { sameSite: "lax" } } },
    },
    plugins: [
      magicLink({
        expiresIn: 600,
        sendMagicLink: async ({ email, url }) => {
          outbox.set(email, url);
        },
      }),
      passkey({ rpID: env.RP_ID, rpName: "Lamina PDF", origin: env.BASE_URL }),
      bearer(),
      deviceAuthorization({ expiresIn: "10m", interval: "1s" }),
    ],
  };
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const opts = options(env);
    const auth = betterAuth(opts);
    if (url.pathname === "/") return new Response(PAGE, { headers: { "content-type": "text/html" } });
    if (url.pathname === "/_spike/migrate" && request.method === "POST") {
      const { runMigrations, toBeCreated } = await getMigrations(opts);
      await runMigrations();
      await env.DB.exec("DROP TABLE IF EXISTS desktop_code");
      await env.DB.exec(
        "CREATE TABLE desktop_code (code TEXT PRIMARY KEY, challenge TEXT NOT NULL, user_id TEXT NOT NULL, expires INTEGER NOT NULL)",
      );
      return Response.json({ created: toBeCreated.map((t) => t.table) });
    }
    if (url.pathname === "/_spike/outbox") {
      return Response.json({ url: outbox.get(url.searchParams.get("email")) ?? null });
    }
    if (url.pathname === "/api/me/export") return exportAccount(request, env, auth);
    if (url.pathname.startsWith("/api/desktop/")) return desktopRoutes(request, env, auth);
    if (url.pathname.startsWith("/api/auth/")) return auth.handler(request);
    return new Response("not found", { status: 404 });
  },
};

/**
 * The rows D1 keeps for the signed-in user, as one JSON file (GDPR art. 20). Left out: pending
 * magic links and desktop codes, which expire within minutes and are deleted when used.
 */
async function exportAccount(request, env, auth) {
  const session = await auth.api.getSession({ headers: request.headers });
  if (session === null) return new Response("sign in first", { status: 401 });
  const id = session.user.id;
  const rows = async (sql) => (await env.DB.prepare(sql).bind(id).all()).results;
  return Response.json({
    user: session.user,
    accounts: await rows(
      "SELECT id, providerId, accountId, createdAt FROM account WHERE userId = ?",
    ),
    sessions: await rows("SELECT id, createdAt, expiresAt, userAgent FROM session WHERE userId = ?"),
    passkeys: await rows("SELECT id, name, deviceType, createdAt FROM passkey WHERE userId = ?"),
    devices: await rows('SELECT id, clientId, status, expiresAt FROM deviceCode WHERE userId = ?'),
  });
}
