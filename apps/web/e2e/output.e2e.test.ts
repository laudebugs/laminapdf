// Print, Download a copy and Document properties in the built reader (todo C6c), in this run's
// browser. Printing a 50-page file with portrait and landscape pages: the frame's print() is
// replaced before it runs, so the test sees what would print (each page's image loaded, its size
// in points, the named @page rules) and no print window opens; neither the page nor the frame
// meets a CSP violation, and nothing is fetched. Download saves the bytes as opened, compared by
// SHA-256, for a local file, a remote `#url=` file and an encrypted file. Properties shows what
// pypdf reads (fixtures/corpus/expected.json), works from the keyboard and passes axe.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import axe from "axe-core";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { type InitScript, load as loadPage, opened, type Watched } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** Pages in points: portrait US Letter, with every fifth page landscape. */
const SIZES: readonly (readonly [number, number])[] = Array.from({ length: 50 }, (_, i) =>
  i % 5 === 4 ? [792, 612] : [612, 792],
);

/** A PDF of `sizes.length` pages, each `[width, height]` points, its number written on it. */
function pagesPdf(sizes: readonly (readonly [number, number])[]): Uint8Array {
  const pages = sizes.length;
  const objects = ["<< /Type /Catalog /Pages 2 0 R >>", ""];
  const kids: string[] = [];
  sizes.forEach(([w, h], i) => {
    const page = objects.length + 1;
    kids.push(`${page} 0 R`);
    const text = `BT /F1 36 Tf 72 ${h - 108} Td (Page ${i + 1}) Tj ET`;
    objects.push(
      `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${w} ${h}] /Contents ${page + 1} 0 R /Resources << /Font << /F1 ${pages * 2 + 3} 0 R >> >> >>`,
      `<< /Length ${text.length} >>\nstream\n${text}\nendstream`,
    );
  });
  objects[1] = `<< /Type /Pages /Kids [${kids.join(" ")}] /Count ${pages} >>`;
  objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
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

/** What the frame held when it would have printed. */
interface Printed {
  readonly images: readonly { loaded: boolean; width: string; height: string; page: string }[];
  readonly rules: readonly string[];
  readonly violations: readonly string[];
}

/**
 * Without either file picker (the file input and plain downloads instead), and with every print
 * frame's print() replaced by one that records the frame and sends `afterprint`.
 */
const CAPTURE: InitScript = {
  arg: [],
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
    const prints: unknown[] = [];
    const violationTargets: string[] = [];
    Object.assign(window, { prints, violationTargets });
    document.addEventListener("securitypolicyviolation", (e) =>
      violationTargets.push(e.target instanceof Element ? e.target.tagName : String(e.target)),
    );
    new MutationObserver((records) => {
      for (const record of records) {
        for (const node of record.addedNodes) {
          if (!(node instanceof HTMLIFrameElement)) continue;
          const win = node.contentWindow;
          const doc = node.contentDocument;
          if (win === null || doc === null) continue;
          const violations: string[] = [];
          doc.addEventListener("securitypolicyviolation", (e) => violations.push(e.violatedDirective));
          win.print = () => {
            const images = [...doc.images].map((img) => ({
              loaded: img.complete && img.naturalWidth > 0,
              width: img.style.width,
              height: img.style.height,
              page: getComputedStyle(img).getPropertyValue("page"),
            }));
            const rules = doc.adoptedStyleSheets.flatMap((sheet) => [...sheet.cssRules].map((r) => r.cssText));
            prints.push({ images, rules, violations });
            win.dispatchEvent(new Event("afterprint"));
          };
        }
      }
    }).observe(document, { childList: true, subtree: true });
  },
};

const sha256 = (bytes: Uint8Array) => createHash("sha256").update(bytes).digest("hex");

describe(`print, download and properties in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("hello.pdf", readFileSync(new URL("hello.pdf", CORPUS)));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  const load = (path = "/", home = true, bypassCSP = false): Promise<Watched> =>
    loadPage(browser, origin, path, CAPTURE, { ignoreHTTPSErrors: true, home, bypassCSP });

  /** Chooses a file with Choose a file and waits for its first page. */
  async function choose(watched: Watched, name: string, bytes: Uint8Array): Promise<void> {
    const chooser = watched.page.waitForEvent("filechooser");
    await watched.page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles({ name, mimeType: "application/pdf", buffer: Buffer.from(bytes) });
    await watched.page.locator(".web-viewer canvas").first().waitFor();
  }

  /** Chooses `label` from the top bar's More menu. */
  async function more(page: Page, label: string): Promise<void> {
    await page.getByRole("button", { name: "More" }).first().click();
    await page.getByRole("menuitem", { name: label }).click();
  }

  /** What the print frames held, in order. */
  const prints = (page: Page) => page.evaluate(() => (window as unknown as { prints: Printed[] }).prints);

  it("prints 50 portrait and landscape pages, each loaded at its own size, with no CSP violation or request", async () => {
    const watched = await load();
    const { page } = watched;
    await choose(watched, "fifty.pdf", pagesPdf(SIZES));
    // The service worker's own downloads of the reader's files while it installs aren't printing
    // (Firefox reports them as the page's), so they are left out, as the network suite does.
    const requests: string[] = [];
    page.on("request", (request) => {
      if (request.serviceWorker() === null && !/^(blob|data):/.test(request.url())) requests.push(request.url());
    });
    await page.keyboard.press("ControlOrMeta+p");
    const dialog = page.getByRole("dialog", { name: "Print" });
    await dialog.waitFor();
    await dialog.getByRole("button", { name: "Print…" }).click();
    await expect.poll(() => prints(page), { timeout: 120_000 }).toHaveLength(1);
    const [printed] = await prints(page);
    expect(printed?.images).toHaveLength(50);
    expect(printed?.images.every((image) => image.loaded)).toBe(true);
    SIZES.forEach(([w, h], i) => {
      expect(printed?.images[i]).toMatchObject({ width: `${w}pt`, height: `${h}pt` });
      expect(printed?.rules.join("\n")).toMatch(new RegExp(`@page p${i} \\{[^}]*size: ${w}pt ${h}pt`));
    });
    expect(printed?.violations).toEqual([]);
    // The frame is gone, with its images.
    await expect.poll(() => page.locator("iframe").count()).toBe(0);
    expect(requests).toEqual([]);
    // Playwright's WebKit reports every native <select> under `style-src 'self'` as an inline style
    // (the engine's own styles for the control: a page with only a bare <select> does the same), so
    // there those are left out; everything else, the print frame's included, must be clean.
    const targets = await page.evaluate(() => (window as unknown as { violationTargets: string[] }).violationTargets);
    expect(BROWSER === "webkit" ? targets.filter((t) => t !== "SELECT") : targets).toEqual([]);
    expect(watched.foreign).toEqual([]);
    await page.close();
  });

  it("prints a range of pages by number, and says which page a file lacks", async () => {
    const watched = await load();
    const { page } = watched;
    await choose(watched, "fifty.pdf", pagesPdf(SIZES));
    await more(page, "Print…");
    const dialog = page.getByRole("dialog", { name: "Print" });
    const field = dialog.getByRole("textbox", { name: "Pages to print" });
    await field.fill("60");
    await dialog.getByRole("button", { name: "Print…" }).click();
    await expect.poll(() => dialog.getByText("There's no page 60. This file has 50 pages.").count()).toBe(1);
    await expect.poll(() => field.evaluate((el) => el === document.activeElement)).toBe(true);
    await field.fill("4-5, 9");
    await field.press("Enter");
    await expect.poll(() => prints(page), { timeout: 60_000 }).toHaveLength(1);
    const [printed] = await prints(page);
    expect(printed?.images.map((image) => image.width)).toEqual(["612pt", "792pt", "612pt"]);
    await page.close();
  });

  it("asks before printing a file whose author restricted printing, then prints it", async () => {
    const watched = await load();
    const { page } = watched;
    const encrypted = new URL("../../../fixtures/encrypted/", import.meta.url);
    const passwords = JSON.parse(readFileSync(new URL("passwords.json", encrypted), "utf8")) as {
      files: Record<string, { user: string }>;
    };
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("r5-aes-256-user.pdf", encrypted).pathname);
    const prompt = page.getByRole("dialog", { name: "Enter the password" });
    await prompt.getByLabel("Password", { exact: true }).fill(passwords.files["r5-aes-256-user.pdf"]?.user ?? "");
    await prompt.getByRole("button", { name: "Open" }).click();
    await page.locator(".web-viewer canvas").first().waitFor();
    await page.keyboard.press("ControlOrMeta+p");
    const dialog = page.getByRole("dialog", { name: "Print" });
    await dialog.getByText("The author restricted printing of this file.").waitFor();
    await dialog.getByRole("button", { name: "Print anyway…" }).click();
    const confirm = page.getByRole("alertdialog", { name: "Print a restricted file?" });
    await confirm.waitFor();
    await expect
      .poll(() => confirm.getByRole("button", { name: "Cancel" }).evaluate((el) => el === document.activeElement))
      .toBe(true);
    expect(await prints(page)).toEqual([]);
    await confirm.getByRole("button", { name: "Print anyway" }).click();
    await expect.poll(() => prints(page), { timeout: 60_000 }).toHaveLength(1);
    expect((await prints(page))[0]?.images).toHaveLength(2);
    await page.close();
  });

  it("prints a range given by page labels", async () => {
    const watched = await load();
    const { page } = watched;
    await choose(
      watched,
      "labelled.pdf",
      readFileSync(new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url)),
    );
    await more(page, "Print…");
    const dialog = page.getByRole("dialog", { name: "Print" });
    // The file's pages are labelled i, ii, 1 and 2: "ii-1" is its second and third pages.
    await dialog.getByRole("textbox", { name: "Pages to print" }).fill("ii-1");
    await dialog.getByRole("button", { name: "Print…" }).click();
    await expect.poll(() => prints(page), { timeout: 60_000 }).toHaveLength(1);
    expect((await prints(page))[0]?.images).toHaveLength(2);
    await page.close();
  });

  it("stops preparing on Cancel, and opens no print window", async () => {
    const watched = await load();
    const { page } = watched;
    await choose(watched, "fifty.pdf", pagesPdf(SIZES));
    await more(page, "Print…");
    const dialog = page.getByRole("dialog", { name: "Print" });
    await dialog.getByRole("combobox", { name: "Quality" }).selectOption("high");
    await dialog.getByRole("button", { name: "Print…" }).click();
    const progress = page.getByRole("dialog", { name: "Preparing pages for print" });
    await progress.waitFor();
    await progress.getByRole("button", { name: "Cancel" }).click();
    await progress.waitFor({ state: "detached" });
    await expect.poll(() => page.locator("iframe").count()).toBe(0);
    expect(await prints(page)).toEqual([]);
    await page.close();
  });

  /** Download a copy's file, by SHA-256. */
  async function downloaded(page: Page): Promise<{ name: string; hash: string }> {
    const download = page.waitForEvent("download");
    await more(page, "Download a copy");
    const file = await download;
    const path = await file.path();
    return { name: file.suggestedFilename(), hash: sha256(readFileSync(path)) };
  }

  it("downloads a local file's bytes as opened", async () => {
    const watched = await load();
    const bytes = readFileSync(new URL("outline.pdf", CORPUS));
    await choose(watched, "outline.pdf", bytes);
    expect(await downloaded(watched.page)).toEqual({ name: "outline.pdf", hash: sha256(bytes) });
    await expect.poll(() => watched.page.getByText("Downloaded outline.pdf").count()).toBe(1);
    await watched.page.close();
  });

  it("downloads an encrypted file still encrypted", async () => {
    const watched = await load();
    const bytes = readFileSync(new URL("enc.pdf", CORPUS));
    await choose(watched, "enc.pdf", bytes);
    expect((await downloaded(watched.page)).hash).toBe(sha256(bytes));
    await watched.page.close();
  });

  it("downloads a remote file's bytes as its host sent them", async () => {
    const url = `${host.origin}/cors/ranges/hello.pdf`;
    const watched = await load(`/#url=${encodeURIComponent(url)}`, false);
    await opened(watched, "hello.pdf");
    const bytes = readFileSync(new URL("hello.pdf", CORPUS));
    expect(await downloaded(watched.page)).toEqual({ name: "hello.pdf", hash: sha256(bytes) });
    await watched.page.close();
  });

  it("shows a file's properties as pypdf reads them, from the keyboard, and passes axe", async () => {
    const facts = JSON.parse(readFileSync(new URL("expected.json", CORPUS), "utf8")) as Record<
      string,
      { header_version: string; properties: { title: string; producer: string; fonts: string[] } }
    >;
    const fact = facts["outline.pdf"];
    const watched = await load("/", true, true);
    const { page } = watched;
    await choose(watched, "outline.pdf", readFileSync(new URL("outline.pdf", CORPUS)));
    await more(page, "Document properties…");
    const dialog = page.getByRole("dialog", { name: "Document properties" });
    await dialog.getByRole("heading", { name: "Fonts" }).waitFor();
    const value = (label: string) => dialog.locator("dt", { hasText: new RegExp(`^${label}$`) }).locator("+ dd");
    expect(await value("Title").textContent()).toContain(fact?.properties.title);
    expect(await value("Made with").textContent()).toContain(fact?.properties.producer);
    expect(await value("PDF version").textContent()).toContain(fact?.header_version);
    for (const font of fact?.properties.fonts ?? []) {
      expect(await dialog.getByRole("cell", { name: font }).count()).toBeGreaterThan(0);
    }
    expect(await value("Signatures").textContent()).toContain("None");
    await page.addScriptTag({ content: axe.source });
    const serious = await page.evaluate(async () => {
      const run = (
        window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
      ).axe;
      const result = await run.run(document);
      return result.violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
    });
    expect(serious).toEqual([]);
    await page.keyboard.press("Escape");
    await dialog.waitFor({ state: "detached" });
    await page.close();
  });
});
