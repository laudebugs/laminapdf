// Push buttons and sending a form (E8 plan §2.3), in this run's browser, with no file pickers. A
// button's safe actions run; the rest are named and left alone. A form is sent only after the person
// confirms that one send: no request leaves before Send, the request carries exactly the fields the
// dialog lists, and a send of the whole PDF or to an http: address is refused. The form's server is
// played by a route for https://forms.example.com, so nothing leaves the test.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page, Request, Route } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const ACTIONS = new URL("../../../fixtures/forms/actions.pdf", import.meta.url);
const SERVER = "https://forms.example.com/**";

/** The reader, wide, with the actions form open in Fill & Sign and its controls drawn. */
async function fill(): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
      },
      arg: undefined,
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 900 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(ACTIONS.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Fill & Sign", exact: true }).click();
  await page.getByRole("toolbar", { name: "Fill & Sign tools" }).waitFor();
  await page.locator(".lamina-form-field").first().waitFor({ timeout: 20_000 });
  return watched;
}

/** The push button named `name`. */
const button = (page: Page, name: string) => page.getByRole("button", { name, exact: true });

/** The submit dialog. */
const dialog = (page: Page) => page.getByRole("dialog", { name: "Send this form to another computer?" });

/** The server answers every request, as one that allows requests from web pages does. */
async function serve(page: Page, seen: Request[], status = 200): Promise<void> {
  await page.route(SERVER, async (route: Route) => {
    seen.push(route.request());
    const headers = { "access-control-allow-origin": "*", "access-control-allow-headers": "*" };
    if (route.request().method() === "OPTIONS") return route.fulfill({ status: 204, headers });
    return route.fulfill({ status, headers, body: "ok" });
  });
}

/** Saves the document (a download, with no pickers) and returns the bytes. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** Each field's `/F` flags in `bytes`, by field name, as pdf.js reads them. */
async function flagsOf(bytes: Uint8Array): Promise<Record<string, number>> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const flags: Record<string, number> = {};
    const annotations = (await (await doc.getPage(1)).getAnnotations()) as {
      fieldName?: string;
      annotationFlags?: number;
    }[];
    for (const a of annotations) if (a.fieldName !== undefined) flags[a.fieldName] = a.annotationFlags ?? 0;
    return flags;
  } finally {
    await task.destroy();
  }
}

beforeAll(async () => {
  browser = await launch();
});

afterAll(async () => {
  await browser?.close();
});

describe(`push buttons and form submit (${BROWSER})`, () => {
  it("[cap E.04 web read] runs a page move and says plainly what it doesn't run", async () => {
    const watched = await fill();
    const { page } = watched;
    await button(page, "script").click();
    await expect
      .poll(() => page.locator(".paper-toast").allTextContents())
      .toEqual([expect.stringContaining("This button runs a script, which Lamina PDF doesn’t run.")]);
    await button(page, "launch").click();
    await expect
      .poll(() => page.locator(".paper-toast").allTextContents())
      .toEqual(expect.arrayContaining([expect.stringContaining("opens another program")]));
    await button(page, "import").click();
    await expect
      .poll(() => page.locator(".paper-toast").allTextContents())
      .toEqual(expect.arrayContaining([expect.stringContaining("Import form data")]));
    await button(page, "next").click();
    await expect.poll(() => page.getByRole("textbox", { name: "Page" }).inputValue()).toBe("2");
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("[cap E.10 web round-trip] hides a field and an annotation from a button, in one undo step the saved file keeps", async () => {
    const watched = await fill();
    const { page } = watched;
    await page.getByLabel("notes", { exact: true }).waitFor();
    await button(page, "hide").click();
    await expect.poll(() => page.getByLabel("notes", { exact: true }).count()).toBe(0);
    const flags = await flagsOf(await save(page));
    // The Hidden bit (2) is set on the field's widget.
    expect((flags.notes ?? 0) & 2).toBe(2);
    expect((flags.name ?? 0) & 2).toBe(0);
    await page.locator(".paper-top-bar").getByRole("button", { name: /^Undo/ }).click();
    await page.getByLabel("notes", { exact: true }).waitFor({ timeout: 20_000 });
    await clean(watched);
  });

  it("resets the fields a button names, after asking", async () => {
    const watched = await fill();
    const { page } = watched;
    await button(page, "resetname").click();
    const question = page.getByRole("alertdialog");
    await question.waitFor();
    await question.getByRole("button", { name: "Reset form" }).click();
    await expect.poll(() => page.getByLabel("name", { exact: true }).inputValue()).toBe("");
    expect(await page.getByLabel("email", { exact: true }).inputValue()).toBe("ada@example.com");
    await clean(watched);
  });

  it("[cap E.16 web write] sends nothing before Send, then exactly the fields the dialog lists, with no cookies or referrer", async () => {
    const watched = await fill();
    const { page } = watched;
    const seen: Request[] = [];
    await serve(page, seen);
    await button(page, "sendhtml").click();
    const box = dialog(page);
    await box.waitFor();
    // `empty` is required and still empty: the dialog says so and Send waits.
    expect(await box.textContent()).toContain("Fill the required fields first.");
    expect(await box.getByRole("button", { name: "Send", exact: true }).isDisabled()).toBe(true);
    await box.getByRole("button", { name: /Don.t send/ }).click();
    await box.waitFor({ state: "hidden" });
    expect(watched.foreign).toEqual([]);
    // Answered, the form can go; still nothing leaves until Send.
    const empty = page.getByLabel("empty", { exact: true });
    await empty.fill("yes please");
    await empty.press("Tab");
    await button(page, "sendhtml").click();
    await box.waitFor();
    expect(await box.textContent()).toContain("forms.example.com");
    await box.getByText(/Show all 4 fields and values/).click();
    const rows = await box.locator(".paper-form-submit-fields tr").allTextContents();
    expect(rows).toEqual(["nameAda Lovelace", "emailada@example.com", "notesHello", "emptyyes please"]);
    expect(await box.locator("[role=alert]").count()).toBe(0);
    expect(await serious(page)).toEqual([]);
    expect(watched.foreign).toEqual([]);
    expect(seen).toEqual([]);
    await box.getByRole("button", { name: "Send", exact: true }).click();
    await box.waitFor({ state: "hidden" });
    await expect.poll(() => seen.filter((r) => r.method() === "POST").length).toBe(1);
    const post = seen.find((r) => r.method() === "POST");
    expect(post?.postData()).toBe("name=Ada+Lovelace&email=ada%40example.com&notes=Hello&empty=yes+please");
    expect(post?.headers()["content-type"]).toContain("application/x-www-form-urlencoded");
    expect(post?.headers().referer).toBeUndefined();
    expect(post?.headers().cookie).toBeUndefined();
    // The secret field, a NoExport one, never left.
    expect(post?.postData()).not.toContain("hush");
    await expect
      .poll(() => page.locator(".paper-toast").allTextContents())
      .toEqual(expect.arrayContaining([expect.stringContaining("Sent to forms.example.com")]));
    // The one request went to the host the dialog named, and to no other.
    expect([...new Set(watched.foreign.map((u) => new URL(u).host))]).toEqual(["forms.example.com"]);
    watched.foreign.length = 0;
    await clean(watched);
  });

  it("refuses to send the whole PDF and refuses an http: address, with no Send and no request", async () => {
    const watched = await fill();
    const { page } = watched;
    const seen: Request[] = [];
    await serve(page, seen);
    await button(page, "sendpdf").click();
    const box = dialog(page);
    await box.waitFor();
    expect(await box.locator("[role=alert]").textContent()).toContain("This button would send the whole PDF.");
    expect(await box.getByRole("button", { name: "Send", exact: true }).count()).toBe(0);
    expect(await serious(page)).toEqual([]);
    await box.getByRole("button", { name: /Don.t send/ }).click();
    await box.waitFor({ state: "hidden" });
    await button(page, "sendhttp").click();
    await box.waitFor();
    expect(await box.locator("[role=alert]").textContent()).toContain("only sends to public https:// addresses");
    expect(await box.getByRole("button", { name: "Send", exact: true }).count()).toBe(0);
    expect(await box.getByRole("button", { name: "Export form data…" }).count()).toBe(1);
    await box.getByRole("button", { name: /Don.t send/ }).click();
    expect(seen).toEqual([]);
    expect(watched.foreign).toEqual([]);
    await clean(watched);
  });

  it("says Not sent when the server doesn't accept a send from a web page, and keeps Send to try again", async () => {
    const watched = await fill();
    const { page } = watched;
    // A server with no CORS headers: the browser refuses to hand over the answer.
    await page.route(SERVER, (route) => route.fulfill({ status: 200, body: "ok" }));
    const empty = page.getByLabel("empty", { exact: true });
    await empty.fill("x");
    await empty.press("Tab");
    await button(page, "send").click();
    const box = dialog(page);
    await box.waitFor();
    await box.getByRole("button", { name: "Send", exact: true }).click();
    await expect.poll(() => box.locator("[role=alert]").textContent()).toContain("Not sent.");
    expect(await box.locator("[role=alert]").textContent()).toContain("didn’t allow a send from a web page");
    expect(await box.getByRole("button", { name: "Send", exact: true }).isEnabled()).toBe(true);
    expect(await box.getByRole("button", { name: "Export form data…" }).count()).toBe(1);
    expect(await serious(page)).toEqual([]);
    await box.getByRole("button", { name: /Don.t send/ }).click();
    watched.foreign.length = 0;
    await clean(watched);
  });
});
