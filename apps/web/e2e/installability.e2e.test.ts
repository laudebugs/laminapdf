// The reader as an installable app (todo C7b; R15), in this run's browser. Lighthouse has no PWA
// audit any more, so this checks what installing needs directly: the manifest is valid against
// SchemaStore's schema (pinned in e2e/fixtures/) and has the fields browsers require, every icon
// it names is served as the image and size it claims, the service worker controls the page and
// serves it offline, Chromium itself finds nothing that stops an install, and the worker never
// asks another site for anything.

import { readFileSync } from "node:fs";
import { getLocal } from "@laminapdf/test-utils/wrangler-dev";
import { Ajv } from "ajv";
import type { Browser } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load } from "./reader.ts";

const origin = inject("origin");
const ca = inject("ca");
/** `GET path` from the local server, whose certificate is self-signed. */
const get = (path: string) => getLocal(new URL(path, origin), ca);
const FIXTURES = new URL("./fixtures/", import.meta.url);

/** The manifest's icon entries. */
interface ManifestIcon {
  readonly src: string;
  readonly sizes: string;
  readonly type: string;
  readonly purpose?: string;
}

/** A PNG's width and height, from its IHDR chunk. */
function pngSize(bytes: Uint8Array): string {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return `${view.getUint32(16)}x${view.getUint32(20)}`;
}

describe(`installability in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("serves a valid manifest with the fields installing needs, and every icon it names", async () => {
    const response = await get("/manifest.webmanifest");
    expect(response.status).toBe(200);
    const manifest = (await response.json()) as Record<string, unknown> & { icons: ManifestIcon[] };
    const ajv = new Ajv({ strict: false, allErrors: true });
    for (const name of ["web-manifest", "web-manifest-app-info", "web-manifest-share-target"]) {
      ajv.addSchema(JSON.parse(readFileSync(new URL(`${name}.schema.json`, FIXTURES), "utf8")));
    }
    const validate = ajv.compile(
      JSON.parse(readFileSync(new URL("web-manifest-combined.schema.json", FIXTURES), "utf8")),
    );
    expect(validate(manifest), JSON.stringify(validate.errors)).toBe(true);

    expect(manifest).toMatchObject({ name: "Lamina PDF", short_name: "Lamina PDF", display: "standalone" });
    for (const key of ["start_url", "scope"] as const) {
      expect(new URL(manifest[key] as string, origin).origin, key).toBe(new URL(origin).origin);
    }
    expect(manifest.file_handlers).toEqual([{ action: "/", accept: { "application/pdf": [".pdf"] } }]);
    expect(manifest.icons.map((icon) => icon.sizes)).toEqual(expect.arrayContaining(["192x192", "512x512"]));
    expect(manifest.icons.filter((icon) => icon.purpose === "maskable")).toHaveLength(1);
    for (const icon of manifest.icons) {
      const image = await get(icon.src);
      expect(image.status, icon.src).toBe(200);
      expect(image.headers.get("content-type"), icon.src).toBe(icon.type);
      expect(pngSize(new Uint8Array(await image.arrayBuffer())), icon.src).toBe(icon.sizes);
    }
  });

  it("is controlled by its service worker, served offline, and Chromium finds nothing stopping an install", async () => {
    const watched = await load(browser, origin, "/", undefined, { ignoreHTTPSErrors: true });
    const { page } = watched;
    const workerRequests: string[] = [];
    page.context().on("request", (request) => {
      if (request.serviceWorker() !== null) workerRequests.push(request.url());
    });
    await page.evaluate(() => navigator.serviceWorker.ready);
    await page.reload();
    expect(await page.evaluate(() => navigator.serviceWorker.controller !== null)).toBe(true);
    await page.context().setOffline(true);
    expect((await page.reload())?.fromServiceWorker()).toBe(true);
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    await page.context().setOffline(false);
    if (BROWSER === "chromium") {
      const cdp = await page.context().newCDPSession(page);
      const { installabilityErrors } = (await cdp.send("Page.getInstallabilityErrors")) as {
        installabilityErrors: { errorId: string }[];
      };
      expect(installabilityErrors.map((e) => e.errorId)).toEqual([]);
    }
    // The worker fetches only the reader's own files.
    expect(workerRequests.filter((url) => new URL(url).origin !== new URL(origin).origin)).toEqual([]);
    await page.close();
  });
});
