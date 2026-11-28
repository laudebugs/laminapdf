import { describe, expect, it, vi } from "vitest";
import { submitForm } from "../src/platform/submit.ts";

const body = new Uint8Array([1, 2, 3]);
const post = {
  url: "https://forms.example.com/submit",
  method: "POST",
  contentType: "application/vnd.fdf",
  body,
} as const;

/** A scope whose `fetch` answers `response`, or fails when it is an Error. */
function scope(response: Response | Error) {
  const fetch = vi.fn(async () => {
    if (response instanceof Error) throw response;
    return response;
  });
  return {
    fetch: fetch as unknown as typeof globalThis.fetch,
    location: { origin: "https://app.laminapdf.com" },
    calls: fetch,
  };
}

describe("submitForm over the web", () => {
  it("makes the one request with no cookies, no referrer and no redirect", async () => {
    const s = scope(new Response("", { status: 200 }));
    expect(await submitForm(s, post)).toEqual({ sent: true, status: 200 });
    const [target, init] = s.calls.mock.calls[0] as unknown as [URL, RequestInit];
    expect(target.href).toBe("https://forms.example.com/submit");
    expect(init).toMatchObject({
      method: "POST",
      credentials: "omit",
      referrerPolicy: "no-referrer",
      redirect: "error",
      headers: { "Content-Type": "application/vnd.fdf" },
    });
    expect(init.body).toBe(body);
  });

  it("sends a GET with no body", async () => {
    const s = scope(new Response("", { status: 200 }));
    await submitForm(s, { ...post, method: "GET", body: new Uint8Array() });
    const [, init] = s.calls.mock.calls[0] as unknown as [URL, RequestInit];
    expect(init.body).toBeUndefined();
    expect(init.headers).toBeUndefined();
  });

  it("says why a send didn't go through, without throwing", async () => {
    expect(await submitForm(scope(new Response("", { status: 503 })), post)).toEqual({
      sent: false,
      reason: "status",
      status: 503,
    });
    expect(await submitForm(scope(new TypeError("Failed to fetch")), post)).toEqual({
      sent: false,
      reason: "unreachable",
      mayHaveSent: true,
    });
  });

  it("says a failed send may have reached the server, whatever the format", async () => {
    const down = scope(new TypeError("Failed to fetch"));
    for (const request of [
      { ...post, contentType: "application/x-www-form-urlencoded" },
      { ...post, method: "GET" as const, body: new Uint8Array() },
      post,
    ]) {
      expect(await submitForm(down, request)).toEqual({ sent: false, reason: "unreachable", mayHaveSent: true });
    }
  });

  it("calls onRequest just before the request, and not when it refuses", async () => {
    const seen: string[] = [];
    await submitForm(scope(new Response("", { status: 200 })), post, { onRequest: (u) => seen.push(u) });
    await submitForm(
      scope(new Response("", { status: 200 })),
      { ...post, url: "https://localhost/x" },
      {
        onRequest: (u) => seen.push(u),
      },
    );
    expect(seen).toEqual(["https://forms.example.com/submit"]);
  });

  it("stops when the person cancels, and when no answer comes in time", async () => {
    const hang = {
      fetch: ((_: unknown, init: RequestInit) =>
        new Promise((_, reject) => {
          init.signal?.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
        })) as unknown as typeof globalThis.fetch,
      location: { origin: "https://app.laminapdf.com" },
    };
    const controller = new AbortController();
    const cancelled = submitForm(hang, post, { signal: controller.signal });
    controller.abort();
    expect(await cancelled).toEqual({ sent: false, reason: "cancelled" });
    expect(await submitForm(hang, post, { timeoutMs: 10 })).toEqual({ sent: false, reason: "timeout" });
    expect(await submitForm(hang, post, { signal: AbortSignal.abort() })).toEqual({
      sent: false,
      reason: "cancelled",
    });
  });

  it("never reaches an address that isn't a public https: host of another site", async () => {
    for (const url of [
      "http://forms.example.com/x",
      "https://app.laminapdf.com/api",
      "https://app.laminapdf.com./api",
      "https://APP.LAMINAPDF.COM/api",
      "https://app.laminapdf.com:443/api",
      "https://staging-app.laminapdf.com/api",
      "https://laminapdf.com/",
      "https://evil.app.laminapdf.com/",
      "https://localhost/x",
      "https://LOCALHOST./x",
      "https://a.localhost/x",
      "https://printer.local/x",
      "https://db.internal/x",
      "https://intranet/x",
      "https://127.0.0.1/",
      "https://127.1/",
      "https://2130706433/",
      "https://0x7f000001/",
      "https://0177.0.0.1/",
      "https://10.0.0.1/",
      "https://172.16.5.5/",
      "https://192.168.0.1/",
      "https://169.254.169.254/latest",
      "https://100.64.0.1/",
      "https://0.0.0.0/",
      "https://foo.localhost../",
      "https://printer.local../",
      "https://１２７.０.０.１/",
      "https://127。0。0。1/",
      "https://198.18.0.1/",
      "https://192.0.0.8/",
      "https://192.0.2.1/",
      "https://198.51.100.7/",
      "https://203.0.113.9/",
      "https://240.0.0.1/",
      "https://[::1]/",
      "https://[::]/",
      "https://[fd12::1]/",
      "https://[fe80::1]/",
      "https://[::ffff:127.0.0.1]/",
      "https://[::ffff:10.0.0.1]/",
      "https://user:pw@forms.example.com/",
      "javascript:1",
      "not a url",
    ]) {
      const s = scope(new Response("", { status: 200 }));
      expect(await submitForm(s, { ...post, url }), url).toEqual({ sent: false, reason: "refused" });
      expect(s.calls, url).not.toHaveBeenCalled();
    }
  });

  it("reaches public addresses, in any case and with a trailing dot", async () => {
    for (const url of [
      "https://Forms.Example.com./x",
      "https://8.8.8.8/x",
      "https://[2606:4700::1111]/x",
      "https://172.32.0.1/",
    ]) {
      const s = scope(new Response("", { status: 200 }));
      expect(await submitForm(s, { ...post, url }), url).toEqual({ sent: true, status: 200 });
    }
  });
});
