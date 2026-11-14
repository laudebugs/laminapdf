import type { Browser } from "playwright";
import { afterAll, beforeAll, describe, inject, it } from "vitest";
import { launch } from "./browser.ts";
import { chooseFile, load, opened, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);
const boxes = () =>
  Array.from(document.querySelectorAll(".web-viewer"), (v) => ({
    hidden: v.hasAttribute("hidden"),
    display: getComputedStyle(v).display,
    canvases: v.querySelectorAll("canvas").length,
    parent: v.parentElement?.className ?? "",
  }));
describe("probe", () => {
  let browser: Browser;
  beforeAll(async () => {
    browser = await launch();
  });
  afterAll(async () => {
    await browser?.close();
  });
  it("lists the viewer boxes", async () => {
    const w = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    await chooseFile(w, "outline.pdf");
    await opened(w, "Outline fixture");
    console.log("after first open", JSON.stringify(await w.page.evaluate(boxes)));
    const chooser = w.page.waitForEvent("filechooser");
    await w.page.getByRole("button", { name: "More" }).click();
    await w.page.getByRole("menuitem", { name: "Open another file…" }).click();
    await (await chooser).setFiles(new URL("outline.pdf", CORPUS).pathname);
    for (let i = 0; i < 6; i++) {
      console.log(
        `t${i * 50}ms`,
        JSON.stringify(await w.page.evaluate(boxes)),
        await w.page.locator(".web-viewer canvas").count(),
      );
      await w.page.waitForTimeout(50);
    }
    await opened(w, "Outline fixture");
    console.log("after reopen", JSON.stringify(await w.page.evaluate(boxes)));
  });
});
