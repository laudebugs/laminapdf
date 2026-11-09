// Remote files from `#url=` links, against the local remote host (remote-host.ts): a large file
// opens after reading a small part of it with Range requests; a host without range support is read
// whole; a host without CORS headers gets the explanation, never a proxy; the fragment's format
// (an unencoded link refused, plain http refused by a production build, `page=`); and a change of
// `page` alone never opens the document again.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load as loadPage, type Watched } from "./reader.ts";
import { largePdf, type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);
const BIG = largePdf(50);

/** The link that opens `url`, as the share button writes it. */
const link = (url: string, page?: number) =>
  `/#url=${encodeURIComponent(url)}${page === undefined ? "" : `&page=${page}`}`;

/** The page input's value: the label of the page the viewer shows (outline.pdf's are i, A-1, A-2). */
const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();

describe(`remote files in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("big.pdf", BIG);
    host.add("hello.pdf", readFileSync(new URL("hello.pdf", CORPUS)));
    host.add("outline.pdf", readFileSync(new URL("outline.pdf", CORPUS)));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  const load = (path: string, home = false): Promise<Watched> =>
    loadPage(browser, origin, path, undefined, { ignoreHTTPSErrors: true, home });

  /** Requests to `path` on the host since `from`. */
  const served = (path: string, from = 0) => host.requests.slice(from).filter((r) => r.path === path);

  /** Fails unless the page made no CSP violation and asked no origin but the host; then closes it. */
  async function clean(watched: Watched): Promise<void> {
    expect(await watched.violations()).toEqual([]);
    expect(watched.foreign.filter((url) => new URL(url).origin !== host.origin)).toEqual([]);
    await watched.page.close();
  }

  it("renders page 1 of a 50 MB file after reading less than a quarter of it", async () => {
    const from = host.requests.length;
    const watched = await load(link(`${host.origin}/cors/ranges/big.pdf`));
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => watched.page.title()).toBe("big.pdf · Lamina PDF");
    const sent = served("/cors/ranges/big.pdf", from).reduce((sum, r) => sum + r.sent, 0);
    expect(sent).toBeGreaterThan(0);
    expect(sent).toBeLessThan(BIG.length / 4);
    await clean(watched);
  });

  it("reads a file whole from a host that ignores Range", async () => {
    const from = host.requests.length;
    const watched = await load(link(`${host.origin}/cors/whole/hello.pdf`));
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    const gets = served("/cors/whole/hello.pdf", from).filter((r) => r.method === "GET");
    expect(gets).toHaveLength(1);
    await clean(watched);
  });

  it("explains a host that doesn't let other sites read the file, and links to it", async () => {
    const url = `${host.origin}/plain/hello.pdf`;
    const watched = await load(link(url));
    const { page } = watched;
    await page.getByRole("heading", { name: "This site doesn't let other sites read the file" }).waitFor();
    const open = page.getByRole("link", { name: "Open the link in a new tab" });
    expect(await open.getAttribute("href")).toBe(url);
    expect(await open.getAttribute("rel")).toBe("noreferrer noopener");
    expect(await page.evaluate(() => document.activeElement?.textContent)).toBe("Open the link in a new tab");
    await page.getByText(url.slice(url.indexOf("/plain"))).waitFor();
    await page.getByRole("button", { name: "Try again" }).click();
    await page.getByRole("heading", { name: "This site doesn't let other sites read the file" }).waitFor();
    expect(await watched.violations()).toEqual([]);
    await page.close();
  });

  it("opens a link whose address holds &page= at the address's own page", async () => {
    // The host ignores the query; `page=2` inside it is the remote address's, `page=3` ours.
    const url = `${host.origin}/cors/ranges/outline.pdf?x=1&page=2`;
    const watched = await load(link(url, 3));
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => shownPage(watched.page)).toBe("A-2");
    await clean(watched);
  });

  it("opens a link at its named destination", async () => {
    const watched = await load(`${link(`${host.origin}/cors/ranges/outline.pdf`)}&nameddest=chapter2`);
    await watched.page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => shownPage(watched.page)).toBe("A-1");
    await watched.page.evaluate(() => {
      location.hash = location.hash.replace("nameddest=chapter2", "nameddest=appendix");
    });
    await expect.poll(() => shownPage(watched.page)).toBe("A-2");
    await clean(watched);
  });

  it("moves within the open document when only the page changes, and never opens it again", async () => {
    const path = "/cors/ranges/outline.pdf";
    const from = host.requests.length;
    const watched = await load(link(`${host.origin}${path}`));
    const { page } = watched;
    await page.locator(".web-viewer canvas").first().waitFor();
    const probes = () => served(path, from).filter((r) => r.headers.range === "bytes=-65536").length;
    expect(probes()).toBe(1);
    await page.evaluate(() => {
      location.hash = `${location.hash}&page=2`;
    });
    await expect.poll(() => shownPage(page)).toBe("A-1");
    await page.goBack();
    await expect.poll(() => page.evaluate(() => location.hash.includes("page="))).toBe(false);
    await page.goForward();
    await expect.poll(() => shownPage(page)).toBe("A-1");
    expect(probes()).toBe(1);
    await clean(watched);
  });

  it("refuses a link whose address isn't encoded, instead of guessing where it ends", async () => {
    const watched = await load(`/#url=${host.origin}/cors/ranges/hello.pdf?x=1&page=2`);
    const { page } = watched;
    await page.getByText("This link's document address isn't encoded").waitFor();
    expect(host.requests.some((r) => r.path.includes("hello.pdf?x=1"))).toBe(false);
    await page.getByRole("button", { name: "Open a file instead" }).click();
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    // The link is dropped from the address, so a reload shows the home screen.
    await expect.poll(() => page.evaluate(() => location.hash.includes("url="))).toBe(false);
    await clean(watched);
  });

  it("refuses plain http, even to localhost, in a production build", async () => {
    const watched = await load(link("http://localhost:9/doc.pdf"));
    await watched.page.getByText("This link points to an http:// address.").waitFor();
    await clean(watched);
  });
});
