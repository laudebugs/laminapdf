// The gallery loads everything from its own origin (todo U6f, complementing U6b): no font CDN, no
// other host, on any view.

import { expect, test } from "@playwright/test";
import { open, VIEWS } from "./views.ts";

test.skip(({ browserName }) => browserName !== "chromium", "one engine is enough for request origins");

test("every view loads only from the gallery's origin", async ({ page, baseURL }) => {
  const origin = new URL(baseURL ?? "http://127.0.0.1:4174/").origin;
  const elsewhere: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol.startsWith("http") && url.origin !== origin) elsewhere.push(request.url());
  });
  for (const view of VIEWS) await open(page, view);
  expect(elsewhere).toEqual([]);
});

test("the UI font comes from the gallery's own files", async ({ page }) => {
  const fonts: string[] = [];
  page.on("response", (response) => {
    if (response.request().resourceType() === "font") fonts.push(new URL(response.url()).pathname);
  });
  await open(page, { route: "buttons", state: "", pageRelated: false, name: "buttons" });
  expect(fonts.some((path) => path.includes("lamina-sans"))).toBe(true);
});
