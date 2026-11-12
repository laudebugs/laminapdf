// @clerk/backend 3.26.4 on a Worker (H1 prototype c): the server half of a Clerk sign-in, a
// session token (the `__session` cookie Clerk's browser SDK sets) verified with the instance's
// public key, no call to Clerk. Without a Clerk account the spike signs its own tokens with a
// key pair (check.mjs) and gives the Worker the public half, as the dashboard's "JWT public key".

import { verifyToken } from "@clerk/backend";

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (url.pathname !== "/api/me") return new Response("not found", { status: 404 });
    const token = (request.headers.get("authorization") ?? "").replace(/^Bearer /, "");
    try {
      const claims = await verifyToken(token, {
        jwtKey: env.CLERK_JWT_KEY,
        authorizedParties: [env.APP_ORIGIN],
      });
      return Response.json({ userId: claims.sub });
    } catch (error) {
      return Response.json({ error: error.reason ?? String(error) }, { status: 401 });
    }
  },
};
