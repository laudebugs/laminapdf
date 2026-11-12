// Desktop sign-in through the system browser (RFC 8252): the app listens on a loopback port,
// opens /api/desktop/authorize in the browser with a PKCE challenge (RFC 7636), and gets back a
// one-time code it trades, with the verifier, for a session of its own. The browser's session
// cookie never leaves the browser; the app holds a bearer token in the OS keychain.

/** Only a loopback address may receive a code (RFC 8252 §7.3), on any port. */
function loopback(uri) {
  try {
    const url = new URL(uri);
    return url.protocol === "http:" && (url.hostname === "127.0.0.1" || url.hostname === "[::1]");
  } catch {
    return false;
  }
}

async function sha256url(text) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
  return btoa(String.fromCharCode(...new Uint8Array(digest)))
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replace(/=+$/, "");
}

export async function desktopRoutes(request, env, auth) {
  const url = new URL(request.url);
  if (url.pathname === "/api/desktop/authorize" && request.method === "GET") {
    const redirect = url.searchParams.get("redirect_uri") ?? "";
    const challenge = url.searchParams.get("code_challenge") ?? "";
    const state = url.searchParams.get("state") ?? "";
    if (!loopback(redirect) || !/^[A-Za-z0-9_-]{43}$/.test(challenge) || state === "") {
      return new Response("bad request", { status: 400 });
    }
    const session = await auth.api.getSession({ headers: request.headers });
    // A real page would offer sign-in here and come back; the spike asks the browser to sign in first.
    if (session === null) return new Response("sign in first", { status: 401 });
    const { token } = await auth.api.generateOneTimeToken({ headers: request.headers });
    await env.DB.prepare("INSERT INTO desktop_code (code, challenge, expires) VALUES (?, ?, ?)")
      .bind(token, challenge, Date.now() + 120_000)
      .run();
    const back = new URL(redirect);
    back.searchParams.set("code", token);
    back.searchParams.set("state", state);
    return Response.redirect(back.href, 302);
  }
  if (url.pathname === "/api/desktop/token" && request.method === "POST") {
    const { code, code_verifier: verifier } = await request.json();
    const row = await env.DB.prepare("DELETE FROM desktop_code WHERE code = ? RETURNING challenge, expires")
      .bind(code ?? "")
      .first();
    if (row === null || row.expires < Date.now() || (await sha256url(verifier ?? "")) !== row.challenge) {
      return Response.json({ error: "invalid_grant" }, { status: 400 });
    }
    // Consumes the one-time token; it fails a second time and after its expiry.
    const verified = await auth.api.verifyOneTimeToken({ body: { token: code } });
    const context = await auth.$context;
    const session = await context.internalAdapter.createSession(verified.user.id, false, {
      userAgent: "Lamina PDF desktop (spike)",
    });
    return Response.json({ token: session.token, expiresAt: session.expiresAt });
  }
  return new Response("not found", { status: 404 });
}
