// Auth.js (@auth/core 0.41.3) on a Worker with D1 through @auth/d1-adapter (H1 prototype b).
// Throwaway spike code; the README reports what check.mjs found.

import { Auth } from "@auth/core";
import GitHub from "@auth/core/providers/github";
import Passkey from "@auth/core/providers/passkey";
import { D1Adapter, up } from "@auth/d1-adapter";

/** The last sign-in link per address: the spike's stand-in for an email provider. */
const outbox = new Map();

const cookie = (name, sameSite = "strict") => ({
  name: `__Secure-lamina.${name}`,
  options: { httpOnly: true, secure: true, sameSite, path: "/api" },
});

function config(env) {
  return {
    adapter: D1Adapter(env.DB),
    secret: env.AUTH_SECRET,
    basePath: "/api/auth",
    trustHost: true,
    session: { strategy: "database", maxAge: 60 * 60 * 24 * 30 },
    providers: [
      // The email provider type with our own sender: Nodemailer (the documented one) needs SMTP sockets.
      {
        id: "email",
        type: "email",
        name: "Email",
        maxAge: 600,
        sendVerificationRequest: async ({ identifier, url }) => {
          outbox.set(identifier, url);
        },
      },
      GitHub({ clientId: env.GITHUB_CLIENT_ID, clientSecret: env.GITHUB_CLIENT_SECRET }),
      // With the D1 adapter this fails every request (README: MissingAdapterMethods); PASSKEY=1 shows it.
      ...(env.PASSKEY === "1" ? [Passkey] : []),
    ],
    experimental: { enableWebAuthn: env.PASSKEY === "1" },
    cookies: {
      sessionToken: cookie("session-token"),
      csrfToken: cookie("csrf-token"),
      callbackUrl: cookie("callback-url"),
      state: cookie("state", env.STRICT_STATE === "1" ? "strict" : "lax"),
      pkceCodeVerifier: cookie("pkce", env.STRICT_STATE === "1" ? "strict" : "lax"),
      nonce: cookie("nonce", env.STRICT_STATE === "1" ? "strict" : "lax"),
      webauthnChallenge: cookie("webauthn"),
    },
  };
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname === "/_spike/migrate" && request.method === "POST") {
      await up(env.DB);
      return Response.json({ ok: true });
    }
    if (url.pathname === "/_spike/outbox") {
      return Response.json({ url: outbox.get(url.searchParams.get("email")) ?? null });
    }
    if (url.pathname === "/api/me" && request.method === "DELETE") return deleteAccount(request, env);
    if (url.pathname.startsWith("/api/auth")) return Auth(request, config(env));
    return new Response("not found", { status: 404 });
  },
};

/** Auth.js has no deletion route: the app writes one over the adapter. */
async function deleteAccount(request, env) {
  const session = await (await Auth(new Request(new URL("/api/auth/session", request.url), { headers: request.headers }), config(env))).json();
  if (session === null) return new Response("sign in first", { status: 401 });
  const adapter = D1Adapter(env.DB);
  const user = await adapter.getUserByEmail(session.user.email);
  if (user === null) return new Response("no such user", { status: 404 });
  await adapter.deleteUser(user.id);
  return Response.json({ deleted: user.id });
}
