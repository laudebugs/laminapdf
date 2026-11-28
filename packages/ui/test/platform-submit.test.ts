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
    });
  });

  it("never reaches an address that isn't another origin's https: address", async () => {
    for (const url of ["http://forms.example.com/x", "https://app.laminapdf.com/api", "javascript:1", "not a url"]) {
      const s = scope(new Response("", { status: 200 }));
      expect(await submitForm(s, { ...post, url })).toEqual({ sent: false, reason: "refused" });
      expect(s.calls).not.toHaveBeenCalled();
    }
  });
});
