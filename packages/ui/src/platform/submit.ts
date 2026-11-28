// A form's confirmed submit over the web: one `fetch` from the page. No cookies or other
// credentials, no referrer, no redirect followed, only to a public `https:` host that is not the
// app's own (ADR 0011 §3: our origin never receives a body), a local name or a private address.
// The server must allow cross-origin requests (CORS), which many form servers don't:
// `canSubmitWithoutCors` is false here, and a send that fails says `unreachable`.

import type { SubmitFormOptions, SubmitFormRequest, SubmitFormResult } from "./types.js";
import type { WebScope } from "./web.js";

/** How long a send waits for an answer when the caller gives no time. */
const TIMEOUT_MS = 30_000;
/** The media types a browser sends without asking the server first (a CORS "simple" request). */
const SIMPLE = /^(application\/x-www-form-urlencoded|multipart\/form-data|text\/plain)\b/i;
/** Domains that are the app's own: its site and every host under it. */
const OWN_DOMAINS = ["laminapdf.com"];

/** `host` without a trailing dot. */
function bare(host: string): string {
  return host.endsWith(".") ? host.slice(0, -1) : host;
}

/** Whether an IPv4 address (as its four bytes) is loopback, private, link-local, shared or reserved. */
function privateV4([a = 0, b = 0]: readonly number[]): boolean {
  return (
    a === 0 ||
    a === 10 ||
    a === 127 ||
    (a === 100 && b >= 64 && b < 128) ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b < 32) ||
    (a === 192 && b === 168) ||
    a >= 224
  );
}

/** The eight 16-bit groups of the IPv6 address `inner` (no brackets), or undefined when it isn't one. */
function groupsOf(inner: string): number[] | undefined {
  const halves = inner.split("::");
  if (halves.length > 2) return undefined;
  const read = (text: string): number[] | undefined => {
    if (text === "") return [];
    const out: number[] = [];
    for (const part of text.split(":")) {
      if (!/^[0-9a-f]{1,4}$/i.test(part)) return undefined;
      out.push(Number.parseInt(part, 16));
    }
    return out;
  };
  const head = read(halves[0] ?? "");
  const tail = halves.length === 2 ? read(halves[1] ?? "") : [];
  if (head === undefined || tail === undefined) return undefined;
  if (halves.length === 1) return head.length === 8 ? head : undefined;
  const gap = 8 - head.length - tail.length;
  return gap < 1 ? undefined : [...head, ...new Array<number>(gap).fill(0), ...tail];
}

/** Whether an IPv6 address is loopback, unspecified, unique local, link-local or multicast, or embeds a private IPv4 one. */
function privateV6(groups: readonly number[]): boolean {
  const [g0 = 0, g1 = 0, g2 = 0, g3 = 0, g4 = 0, g5 = 0, g6 = 0, g7 = 0] = groups;
  const embedded = privateV4([g6 >> 8, g6 & 255, g7 >> 8, g7 & 255]);
  const zeros = (n: number) => groups.slice(0, n).every((g) => g === 0);
  return (
    zeros(8) ||
    (zeros(7) && g7 === 1) ||
    (g0 & 0xfe00) === 0xfc00 ||
    (g0 & 0xffc0) === 0xfe80 ||
    (g0 & 0xff00) === 0xff00 ||
    (zeros(5) && g5 === 0xffff && embedded) ||
    (zeros(6) && embedded) ||
    (g0 === 0x64 && g1 === 0xff9b && g2 === 0 && g3 === 0 && g4 === 0 && g5 === 0 && embedded)
  );
}

/**
 * Why a send to `target` is refused, or undefined when it may go: it isn't `https:`, has user
 * information, is the app's own host (in any spelling a browser reads as the same name: case, a
 * trailing dot, a host under the app's domain), or is local or private (`localhost`, `.local`,
 * `.internal`, a name with no dot, an address in a loopback, private, link-local, shared or
 * reserved range). A `URL` has already read an IPv4 address in decimal, octal or hex as one
 * address, so `127.1` and `2130706433` are `127.0.0.1` here.
 */
function refusalOf(target: URL, appHost: string | undefined): string | undefined {
  if (target.protocol !== "https:") return "scheme";
  if (target.username !== "" || target.password !== "") return "credentials";
  const host = bare(target.hostname);
  if (host.startsWith("[")) {
    const groups = groupsOf(host.slice(1, -1));
    return groups === undefined || privateV6(groups) ? "private" : undefined;
  }
  if (host === "" || !host.includes(".") || /\.(local|localhost|internal)$/.test(host) || host === "localhost") {
    return "local";
  }
  const v4 = /^(\d+)\.(\d+)\.(\d+)\.(\d+)$/.exec(host);
  if (v4 !== null && privateV4(v4.slice(1).map(Number))) return "private";
  const own = [...OWN_DOMAINS, ...(appHost === undefined ? [] : [bare(appHost).toLowerCase()])];
  if (own.some((domain) => host === domain || host.endsWith(`.${domain}`))) return "own";
  return undefined;
}

/** Makes `request`, which the person confirmed, and says how it ended; it never throws for a failed send. */
export async function submitForm(
  scope: Pick<WebScope, "fetch" | "location">,
  request: SubmitFormRequest,
  options: SubmitFormOptions = {},
): Promise<SubmitFormResult> {
  let target: URL;
  try {
    target = new URL(request.url);
  } catch {
    return { sent: false, reason: "refused" };
  }
  const appHost = scope.location?.hostname ?? hostOf(scope.location?.origin);
  if (
    refusalOf(target, appHost) !== undefined ||
    target.origin === scope.location?.origin ||
    scope.fetch === undefined
  ) {
    return { sent: false, reason: "refused" };
  }
  const cancelled = () => options.signal?.aborted === true;
  if (cancelled()) return { sent: false, reason: "cancelled" };
  const controller = new AbortController();
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, options.timeoutMs ?? TIMEOUT_MS);
  const cancel = () => controller.abort();
  options.signal?.addEventListener("abort", cancel, { once: true });
  const post = request.method === "POST";
  try {
    options.onRequest?.(target.href);
    const response = await scope.fetch(target, {
      method: request.method,
      credentials: "omit",
      referrerPolicy: "no-referrer",
      redirect: "error",
      cache: "no-store",
      mode: "cors",
      signal: controller.signal,
      ...(post ? { headers: { "Content-Type": request.contentType }, body: request.body as BodyInit } : {}),
    });
    return response.ok
      ? { sent: true, status: response.status }
      : { sent: false, reason: "status", status: response.status };
  } catch {
    if (cancelled()) return { sent: false, reason: "cancelled" };
    if (timedOut) return { sent: false, reason: "timeout" };
    return { sent: false, reason: "unreachable", mayHaveSent: !post || SIMPLE.test(request.contentType) };
  } finally {
    clearTimeout(timer);
    options.signal?.removeEventListener("abort", cancel);
  }
}

/** The host of `origin`, when it is one. */
function hostOf(origin: string | undefined): string | undefined {
  try {
    return origin === undefined ? undefined : new URL(origin).hostname;
  } catch {
    return undefined;
  }
}
