// `GET` for the e2e server, which speaks HTTPS with Wrangler's self-signed certificate. The reader's
// policy ends in `upgrade-insecure-requests`, which WebKit applies even to 127.0.0.1, so a plain-HTTP
// server would have every script refused there. Only these requests accept that certificate; Node's
// own checks stay on for everything else.
import type { IncomingHttpHeaders } from "node:http";
import { request } from "node:https";

/** Node's response headers as a `Headers`, a repeated header kept once per value. */
function toHeaders(raw: IncomingHttpHeaders): Headers {
  const headers = new Headers();
  for (const [name, value] of Object.entries(raw)) {
    for (const v of [value].flat()) if (v !== undefined) headers.append(name, v);
  }
  return headers;
}

/** `GET url` from the local server, not following redirects, as a `Response`. */
export function getLocal(url: URL): Promise<Response> {
  return new Promise((resolve, reject) => {
    const req = request(url, { method: "GET", rejectUnauthorized: false }, (res) => {
      const chunks: Buffer[] = [];
      res.on("data", (chunk: Buffer) => chunks.push(chunk));
      res.on("end", () => {
        const status = res.statusCode ?? 0;
        const body = status === 204 || status === 304 ? null : Buffer.concat(chunks);
        resolve(new Response(body, { status, headers: toHeaders(res.headers) }));
      });
      res.on("error", reject);
    });
    req.on("error", reject);
    req.end();
  });
}
