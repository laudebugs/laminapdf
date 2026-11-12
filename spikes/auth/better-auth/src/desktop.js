// Desktop sign-in through the system browser (RFC 8252): the app listens on a loopback port and
// opens /api/desktop/authorize in the browser with a PKCE challenge (RFC 7636). The signed-in
// user approves it on that page; the approval sends a one-time code to the loopback port, and the
// app trades the code and its verifier for a session of its own. The browser's session cookie
// never leaves the browser, and the code is ours (not Better Auth's one-time token, which its own
// /one-time-token/verify route would trade for the browser's session without a verifier).

/** Only a loopback address may receive a code (RFC 8252 §7.3), on any port. */
function loopback(uri) {
  try {
    const url = new URL(uri);
    return url.protocol === "http:" && (url.hostname === "127.0.0.1" || url.hostname === "[::1]");
  } catch {
    return false;
  }
}

const b64url = (bytes) =>
  btoa(String.fromCharCode(...bytes)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");

async function sha256url(text) {
  return b64url(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text))));
}

/** The request's parameters, or `null` when they aren't a loopback redirect, a challenge and a state. */
function authorizeParams(params) {
  const redirect = params.get("redirect_uri") ?? "";
  const challenge = params.get("code_challenge") ?? "";
  const state = params.get("state") ?? "";
  if (!loopback(redirect) || !/^[A-Za-z0-9_-]{43}$/.test(challenge) || state === "") return null;
  return { redirect, challenge, state };
}

const escape = (s) => s.replace(/[&<>"]/g, (c) => `&#${c.charCodeAt(0)};`);

/** The approval page: a local program can open this page, but only the user's click sends a code. */
function approvalPage({ redirect, challenge, state }, email) {
  const field = (name, value) => `<input type="hidden" name="${name}" value="${escape(value)}">`;
  return `<!doctype html><meta charset="utf-8"><title>Sign in Lamina PDF</title>
<h1>Sign in Lamina PDF on this computer?</h1>
<p>Lamina PDF for desktop asks to sign in as ${escape(email)}. Approve only if you just started this from the app.</p>
<form method="post" action="/api/desktop/approve">
${field("redirect_uri", redirect)}${field("code_challenge", challenge)}${field("state", state)}
<button type="submit">Sign in</button></form>`;
}

export async function desktopRoutes(request, env, auth) {
  const url = new URL(request.url);
  if (url.pathname === "/api/desktop/authorize" && request.method === "GET") {
    const params = authorizeParams(url.searchParams);
    if (params === null) return new Response("bad request", { status: 400 });
    const session = await auth.api.getSession({ headers: request.headers });
    // A real page would offer sign-in here and come back; the spike asks the browser to sign in first.
    if (session === null) return new Response("sign in first", { status: 401 });
    return new Response(approvalPage(params, session.user.email), {
      headers: { "content-type": "text/html", "content-security-policy": "default-src 'none'; form-action 'self' http://127.0.0.1:* http://[::1]:*" },
    });
  }
  if (url.pathname === "/api/desktop/approve" && request.method === "POST") {
    // The form posts from our own page: a cross-site post carries no Strict session cookie, and the
    // Origin header must be ours as well.
    if (request.headers.get("origin") !== env.BASE_URL) return new Response("forbidden", { status: 403 });
    const params = authorizeParams(new URLSearchParams(await request.text()));
    if (params === null) return new Response("bad request", { status: 400 });
    const session = await auth.api.getSession({ headers: request.headers });
    if (session === null) return new Response("sign in first", { status: 401 });
    const code = b64url(crypto.getRandomValues(new Uint8Array(32)));
    await env.DB.prepare("INSERT INTO desktop_code (code, challenge, user_id, expires) VALUES (?, ?, ?, ?)")
      .bind(code, params.challenge, session.user.id, Date.now() + 120_000)
      .run();
    const back = new URL(params.redirect);
    back.searchParams.set("code", code);
    back.searchParams.set("state", params.state);
    return Response.redirect(back.href, 303);
  }
  if (url.pathname === "/api/desktop/token" && request.method === "POST") {
    const { code, code_verifier: verifier } = await request.json();
    // DELETE … RETURNING makes the code single-use, whether or not the verifier matches.
    const row = await env.DB.prepare(
      "DELETE FROM desktop_code WHERE code = ? RETURNING challenge, user_id, expires",
    )
      .bind(code ?? "")
      .first();
    if (row === null || row.expires < Date.now() || (await sha256url(verifier ?? "")) !== row.challenge) {
      return Response.json({ error: "invalid_grant" }, { status: 400 });
    }
    const context = await auth.$context;
    const session = await context.internalAdapter.createSession(row.user_id, false, {
      userAgent: "Lamina PDF desktop (spike)",
    });
    return Response.json({ token: session.token, expiresAt: session.expiresAt });
  }
  return new Response("not found", { status: 404 });
}
