// A form's confirmed submit over the web: one `fetch` from the page. No cookies or other
// credentials, no referrer, no redirect followed, only to an `https:` address that is not the app's
// own origin (ADR 0011 §3: our origin never receives a body). The server must allow cross-origin
// requests (CORS), which many form servers don't: `canSubmitWithoutCors` is false here, and a send
// that fails says `unreachable`.

import type { SubmitFormRequest, SubmitFormResult } from "./types.js";
import type { WebScope } from "./web.js";

/** Makes `request`, which the person confirmed, and says how it ended; it never throws for a failed send. */
export async function submitForm(
  scope: Pick<WebScope, "fetch" | "location">,
  request: SubmitFormRequest,
): Promise<SubmitFormResult> {
  let target: URL;
  try {
    target = new URL(request.url);
  } catch {
    return { sent: false, reason: "refused" };
  }
  if (target.protocol !== "https:" || target.origin === scope.location?.origin || scope.fetch === undefined) {
    return { sent: false, reason: "refused" };
  }
  try {
    const post = request.method === "POST";
    const response = await scope.fetch(target, {
      method: request.method,
      credentials: "omit",
      referrerPolicy: "no-referrer",
      redirect: "error",
      cache: "no-store",
      mode: "cors",
      ...(post ? { headers: { "Content-Type": request.contentType }, body: request.body as BodyInit } : {}),
    });
    return response.ok
      ? { sent: true, status: response.status }
      : { sent: false, reason: "status", status: response.status };
  } catch {
    return { sent: false, reason: "unreachable" };
  }
}
