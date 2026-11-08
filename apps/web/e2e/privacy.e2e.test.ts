// ADR 0011's privacy guarantees, checked on what leaves the browser while the reader opens a local
// file, a remote file and an encrypted file of each kind. Every request of the browser context
// (the page's and its document worker's) is recorded, and the remote host logs what reached it:
//
// - no request body carries the document's bytes (any 32-byte window of the file) or `%PDF-`;
// - no request to the reader's own origin names the remote address, an encoded form of it or its
//   host, in its URL or any header, and each is a GET for a static file (no POST, no /api);
// - the remote host sees no Referer and no cookies;
// - no cookie is set or sent, and no origin but the reader's and the chosen host is contacted;
// - a password never appears in any request.

import { readFileSync } from "node:fs";
import type { Browser, BrowserContext, Page, Request, Response } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { type HostRequest, type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const ENCRYPTED = new URL("../../../fixtures/encrypted/", import.meta.url);
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);
const WINDOW = 32;

/** One request the browser made, with what it sent. */
interface Sent {
  readonly method: string;
  readonly url: string;
  readonly headers: Readonly<Record<string, string>>;
  readonly body: Buffer | null;
}

/** Every 32-byte window of `file`, as hex, for finding any of them in a body. */
function windows(file: Uint8Array): Set<string> {
  const set = new Set<string>();
  for (let i = 0; i + WINDOW <= file.length; i++) set.add(Buffer.from(file.subarray(i, i + WINDOW)).toString("hex"));
  return set;
}

/** Whether `body` holds any of `file`'s windows, or `%PDF-`. */
function carries(body: Buffer, file: Set<string>): boolean {
  if (body.includes("%PDF-")) return true;
  for (let i = 0; i + WINDOW <= body.length; i++) {
    if (file.has(body.subarray(i, i + WINDOW).toString("hex"))) return true;
  }
  return false;
}

/** A browser context that records every request and every response's Set-Cookie. */
async function recorded(browser: Browser): Promise<{ context: BrowserContext; sent: Sent[]; cookies: string[] }> {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const sent: Sent[] = [];
  const cookies: string[] = [];
  context.on("request", (request: Request) => {
    sent.push({
      method: request.method(),
      url: request.url(),
      headers: request.headers(),
      body: request.postDataBuffer(),
    });
  });
  context.on("response", (response: Response) => {
    const cookie = response.headers()["set-cookie"];
    if (cookie !== undefined) cookies.push(cookie);
  });
  return { context, sent, cookies };
}

describe(`privacy in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;
  const hello = readFileSync(new URL("hello.pdf", CORPUS));
  const locked = readFileSync(new URL("r4-aes-128-user.pdf", ENCRYPTED));

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("hello.pdf", hello);
    host.add("locked.pdf", locked);
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  /**
   * Opens the reader at `path`, runs `act` on its page, and checks every guarantee for `file`
   * (whose bytes no body may carry), `remote` (the address no request to our origin may name) and
   * `password` (which no request may carry).
   */
  async function check(
    path: string,
    file: Uint8Array,
    act: (page: Page) => Promise<void>,
    { remote, password }: { remote?: string; password?: string } = {},
  ): Promise<void> {
    const { context, sent, cookies } = await recorded(browser);
    const from = host.requests.length;
    const page = await context.newPage();
    await page.goto(new URL(path, origin).href);
    await act(page);
    await page.locator(".web-viewer canvas").first().waitFor();
    // Let the worker's prefetches finish, so they are recorded too.
    await page.waitForLoadState("networkidle");
    const reached = host.requests.slice(from);
    expectNoDocumentBytes(sent, file);
    expectOwnRequests(sent, remote);
    if (remote !== undefined) expectRemoteReached(sent, reached);
    expectHostSawNoReferrerOrCookies(reached);
    expect(sent.map((r) => new URL(r.url).origin).filter((o) => o !== origin && o !== host.origin)).toEqual([]);
    expect(await context.cookies()).toEqual([]);
    expect(cookies).toEqual([]);
    for (const request of sent) expect(request.headers.cookie).toBeUndefined();
    if (password !== undefined) expectNoPassword(sent, reached, password);
    await context.close();
  }

  /** No body carries a 32-byte window of `file`, or `%PDF-`. */
  function expectNoDocumentBytes(sent: readonly Sent[], file: Uint8Array): void {
    const bytes = windows(file);
    for (const request of sent) {
      if (request.body !== null) expect(carries(request.body, bytes), `${request.method} ${request.url}`).toBe(false);
    }
  }

  /** Requests to our origin are GETs for static files that never name `remote` in any form. */
  function expectOwnRequests(sent: readonly Sent[], remote: string | undefined): void {
    const ours = sent.filter((r) => new URL(r.url).origin === origin);
    expect(ours.length).toBeGreaterThan(0);
    const forms =
      remote === undefined ? [] : [remote, encodeURIComponent(remote), encodeURI(remote), new URL(remote).host];
    for (const request of ours) {
      expect(request.method, request.url).toBe("GET");
      expect(new URL(request.url).pathname.startsWith("/api")).toBe(false);
      // The fragment never reaches the server: the browser sends the URL without it.
      const line = [request.url.split("#")[0], ...Object.values(request.headers)].join("\n");
      for (const form of forms) expect(line.includes(form), `${request.url} names ${form}`).toBe(false);
    }
  }

  /** The host was reached, and the recording saw those requests (the document worker's fetches). */
  function expectRemoteReached(sent: readonly Sent[], reached: readonly HostRequest[]): void {
    expect(reached.length).toBeGreaterThan(0);
    expect(sent.some((r) => new URL(r.url).origin === host.origin)).toBe(true);
  }

  function expectHostSawNoReferrerOrCookies(reached: readonly HostRequest[]): void {
    for (const request of reached) {
      expect(request.headers.referer, request.path).toBeUndefined();
      expect(request.headers.cookie, request.path).toBeUndefined();
    }
  }

  /** `password` is in no URL, header or body the browser sent, nor in anything the host logged. */
  function expectNoPassword(sent: readonly Sent[], reached: readonly HostRequest[], password: string): void {
    for (const request of sent) {
      const all = [request.url, ...Object.values(request.headers), request.body?.toString("latin1") ?? ""].join("\n");
      expect(all.includes(password), request.url).toBe(false);
    }
    for (const request of reached) expect(JSON.stringify(request).includes(password)).toBe(false);
  }

  /** Chooses `file` with Choose a file (the input; showOpenFilePicker's dialog can't be driven). */
  const choose = (file: URL) => async (page: Page) => {
    await page.evaluate(() => Reflect.deleteProperty(window, "showOpenFilePicker"));
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(file.pathname);
  };

  const enter = (password: string) => async (page: Page) => {
    const dialog = page.getByRole("dialog", { name: "Enter the password" });
    await dialog.getByLabel("Password", { exact: true }).fill(password);
    await dialog.getByRole("button", { name: "Open" }).click();
  };

  it("a local file stays in the browser", async () => {
    await check("/", hello, choose(new URL("hello.pdf", CORPUS)));
  });

  it("a remote file is fetched from its host alone, without a referrer or cookies", async () => {
    const remote = `${host.origin}/cors/ranges/hello.pdf`;
    await check(`/#url=${encodeURIComponent(remote)}`, hello, async () => undefined, { remote });
  });

  it("a local encrypted file's password goes nowhere", async () => {
    await check(
      "/",
      locked,
      async (page) => {
        await choose(new URL("r4-aes-128-user.pdf", ENCRYPTED))(page);
        await enter("user-pw")(page);
      },
      { password: "user-pw" },
    );
  });

  it("a remote encrypted file's password goes nowhere, its host included", async () => {
    const remote = `${host.origin}/cors/ranges/locked.pdf`;
    await check(`/#url=${encodeURIComponent(remote)}`, locked, enter("user-pw"), { remote, password: "user-pw" });
  });
});
