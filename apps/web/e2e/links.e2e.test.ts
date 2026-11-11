// Links and the back and forward history in the built reader (todo C6b), in this run's browser.
// fixtures/links/links.pdf (every link kind) is opened from the remote host by a `#url=` link:
// an internal link goes to its page and adds a browser history entry whose fragment keeps `url`,
// the browser's Back and Forward and the reader's own buttons return there without opening the
// file again, and the link's address opens the same document at that page in a new tab. A web link
// (in a PDF made here, pointing at the remote host) asks first: Cancel sends nothing, Open website
// opens a tab with no Referer and no opener. `javascript:` and Launch links do nothing. The page
// passes axe with its links.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load as loadPage, type Watched } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const LINKS = readFileSync(new URL("../../../fixtures/links/links.pdf", import.meta.url));

/** A one-page PDF whose only link opens `uri` (a URI action), with its address as the page's text. */
function webLinkPdf(uri: string): Uint8Array {
  const text = `BT /F1 14 Tf 72 700 Td (${uri.replace(/[()\\]/g, "\\$&")}) Tj ET`;
  const objects = [
    "<< /Type /Catalog /Pages 2 0 R >>",
    "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
    "<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> /Annots [6 0 R] >>",
    `<< /Length ${text.length} >>\nstream\n${text}\nendstream`,
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    `<< /Type /Annot /Subtype /Link /Rect [72 690 540 720] /A << /S /URI /URI (${uri}) >> >>`,
  ];
  let out = "%PDF-1.7\n";
  const offsets: number[] = [];
  objects.forEach((body, i) => {
    offsets.push(out.length);
    out += `${i + 1} 0 obj\n${body}\nendobj\n`;
  });
  const xref = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f\r\n`;
  for (const at of offsets) out += `${String(at).padStart(10, "0")} 00000 n\r\n`;
  out += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(out);
}

/** The page input's value: the page the viewer shows. */
const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();
/** The address's fragment, decoded into its keys. */
const fragment = (page: Page) => Object.fromEntries(new URLSearchParams(new URL(page.url()).hash.slice(1)));

describe(`links and history in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("links.pdf", LINKS);
    host.add("away.pdf", webLinkPdf(`${host.origin}/plain/landing`));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  const fileUrl = (name: string) => `${host.origin}/cors/ranges/${name}`;

  /** `name` opened from the remote host, its first link drawn; `bypassCSP` lets axe in. */
  async function open(name: string, bypassCSP = false): Promise<Watched> {
    const watched = await loadPage(browser, origin, `/#url=${encodeURIComponent(fileUrl(name))}`, undefined, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
      home: false,
      bypassCSP,
    });
    await watched.page.locator(".web-viewer a.lamina-link").first().waitFor();
    return watched;
  }

  /** Requests the page made to the host, for the file `name`. */
  const fileRequests = (name: string) => host.requests.filter((r) => r.path.endsWith(`/${name}`)).length;

  it("goes to an internal link's page, and Back and Forward return without opening again", async () => {
    const watched = await open("links.pdf");
    const { page } = watched;
    await page.getByRole("link", { name: "Link to page 3" }).first().click();
    await expect.poll(() => shownPage(page)).toBe("3");
    expect(fragment(page)).toEqual({ url: fileUrl("links.pdf"), page: "3" });
    // Page 3 has drawn: whatever it needed of the file has been read.
    await page.locator('[data-page-index="2"] canvas').first().waitFor();
    const requests = fileRequests("links.pdf");
    await page.goBack();
    await expect.poll(() => shownPage(page)).toBe("1");
    expect(fragment(page)).toEqual({ url: fileUrl("links.pdf") });
    await page.goForward();
    await expect.poll(() => shownPage(page)).toBe("3");
    expect(fileRequests("links.pdf")).toBe(requests);
    expect(new URL(page.url()).pathname).toBe("/");
    await watched.page.close();
  });

  it("names Back by where it goes, and its button returns there", async () => {
    const { page } = await open("links.pdf");
    const back = page.getByRole("group", { name: "History" }).getByRole("button").first();
    await expect.poll(() => back.isDisabled()).toBe(true);
    // Enter follows a link as a click does.
    await page.getByRole("link", { name: "Goes to the next page" }).focus();
    await page.keyboard.press("Enter");
    await expect.poll(() => shownPage(page)).toBe("2");
    await page.getByRole("button", { name: "Back to page 1" }).click();
    await expect.poll(() => shownPage(page)).toBe("1");
    await page.getByRole("button", { name: "Forward to page 2" }).click();
    await expect.poll(() => shownPage(page)).toBe("2");
    await page.close();
  });

  it("opens an internal link's address on the same document at its page", async () => {
    const { page } = await open("links.pdf");
    const href = await page.getByRole("link", { name: "Link to page 3" }).first().getAttribute("href");
    expect(href).toBe(`/#url=${encodeURIComponent(fileUrl("links.pdf"))}&page=3`);
    // A middle click opens a tab in Chromium only; there, it does.
    if (BROWSER === "chromium") {
      const opened = page.context().waitForEvent("page");
      await page.getByRole("link", { name: "Link to page 3" }).first().click({ button: "middle" });
      const middle = await opened;
      await middle.locator(".web-viewer canvas").first().waitFor();
      await expect.poll(() => shownPage(middle), { timeout: 10_000 }).toBe("3");
      await middle.close();
    }
    // Every engine: read the address and open it in a new page.
    const tab = await loadPage(browser, origin, href ?? "/", undefined, { ignoreHTTPSErrors: true, home: false });
    // The new tab opens the remote file first, then goes to the page.
    await tab.page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => shownPage(tab.page), { timeout: 10_000 }).toBe("3");
    await expect.poll(() => tab.page.title()).toBe("links.pdf · Lamina PDF");
    await tab.page.close();
    await page.close();
  });

  it("asks before a web link opens, and opens it with no Referer and no opener", async () => {
    const { page } = await open("away.pdf");
    const landing = () => host.requests.filter((r) => r.path === "/plain/landing");
    const link = page.getByRole("link", { name: `${host.origin}/plain/landing` });
    await link.click();
    const dialog = page.getByRole("dialog", { name: "Open this website?" });
    await expect.poll(() => dialog.textContent()).toContain(`${host.origin}/plain/landing`);
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await expect.poll(() => dialog.count()).toBe(0);
    expect(landing()).toEqual([]);
    await link.click();
    const popup = page.waitForEvent("popup");
    await page.getByRole("button", { name: "Open website" }).click();
    const opened = await popup;
    await expect.poll(() => landing().length).toBeGreaterThan(0);
    // No Referer, or (WebKit) an empty one: either way the site learns nothing of the reader.
    expect(landing()[0]?.headers.referer ?? "").toBe("");
    // The new tab starts blank and then navigates, which destroys a page script mid-read: read
    // until the page it lands on answers.
    const opener = () => opened.evaluate(() => window.opener === null).catch(() => "navigating");
    await expect.poll(opener, { timeout: 10_000 }).toBe(true);
    await opened.close();
    await page.close();
  });

  it("never runs javascript: or Launch links", async () => {
    const { page } = await open("links.pdf");
    let dialogs = 0;
    page.on("dialog", (d) => {
      dialogs++;
      void d.dismiss();
    });
    const before = page.url();
    for (const name of [
      /doesn't open this kind of link: javascript:/,
      "This link asks to run an action. Lamina PDF never runs these.",
    ]) {
      await page.getByRole("link", { name }).first().click({ force: true });
    }
    expect(dialogs).toBe(0);
    expect(page.url()).toBe(before);
    expect(await page.getByRole("dialog").count()).toBe(0);
    await page.close();
  });

  it("selects the text under a link by a drag that starts in the text outside it", async () => {
    const { page } = await open("links.pdf");
    const input = page.getByRole("textbox", { name: "Page" });
    await input.fill("2");
    await input.press("Enter");
    // Page 2's title has no link; the line under it is a link's label.
    const span = (text: string) => page.locator('[data-page-index="1"] .lamina-text-layer span', { hasText: text });
    await span("Named action: FirstPage").waitFor();
    const from = await span("Page 2: chapter").boundingBox();
    const to = await span("Named action: FirstPage").boundingBox();
    if (from === null || to === null) throw new Error("the text isn't laid out");
    await page.mouse.move(from.x + 2, from.y + from.height / 2);
    await page.mouse.down();
    await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
    await page.mouse.move(to.x + to.width - 2, to.y + to.height / 2, { steps: 8 });
    await page.mouse.up();
    await expect.poll(() => page.evaluate(() => getSelection()?.toString() ?? "")).toContain("Named action: First");
    // The drag selected; it followed no link.
    expect(await shownPage(page)).toBe("2");
    await page.close();
  });

  it("passes axe with the links drawn", async () => {
    const { page } = await open("links.pdf", true);
    const { default: axe } = await import("axe-core");
    await page.addScriptTag({ content: axe.source });
    const serious = await page.evaluate(async () => {
      const run = (
        window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
      ).axe;
      const { violations } = await run.run(document.body);
      return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
    });
    expect(serious).toEqual([]);
    await page.close();
  });
});
