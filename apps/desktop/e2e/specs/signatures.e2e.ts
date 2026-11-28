// Signature images in the desktop app (E10c, Paper G01): Fill & Sign's Add a signature image opens
// the dialog, a typed name is placed on the page as one edit, the file is saved in place, and
// pdf.js reads a Stamp annotation back from it. The picture is not a digital signature, and the
// file holds no signature field. Each test edits a copy of a golden file.
import { copyFileSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { fixture, focusWindow, menuClick, openAndWait } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "signatures");

/** A fresh copy of golden file `name`, to edit and save. */
function copyOf(name: string): string {
  const path = join(dir, name);
  copyFileSync(fixture("fixtures", "golden", "examples", name), path);
  return path;
}

/** The subtypes of the annotations on page 1 of the file at `path`, as pdf.js reads them. */
async function subtypes(path: string): Promise<string[]> {
  const task = getDocument({ data: new Uint8Array(readFileSync(path)), verbosity: 0 });
  try {
    const doc = await task.promise;
    const annotations = (await (await doc.getPage(1)).getAnnotations()) as { subtype: string }[];
    return annotations.map((a) => a.subtype);
  } finally {
    await task.destroy();
  }
}

before(() => {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
});

describe("signature images", () => {
  it("[cap F.01 desktop round-trip] [cap F.02 desktop round-trip] places a typed signature as a stamp, saved in place", async () => {
    const path = copyOf("hello.pdf");
    await openAndWait(path);
    await focusWindow();
    await $("aria/Fill & Sign").click();
    await $("aria/Add a signature image").click();
    await $('[role="dialog"]').waitForDisplayed({ timeoutMsg: "the signature dialog didn't show" });
    await $('[role="tab"]=Type').click();
    const labelled = $('//*[@role="dialog"]//label[normalize-space(.)="Your name"]');
    await labelled.waitForExist({ timeoutMsg: "no field labelled Your name" });
    await $(`[id="${await labelled.getAttribute("for")}"]`).setValue("Amara Okafor");
    await $("button=Place signature").click();
    await browser.waitUntil(async () => (await browser.getTitle()) === "• hello.pdf — Lamina PDF", {
      timeoutMsg: "placing the signature didn't mark hello.pdf unsaved",
    });
    await menuClick("cmd:file.save");
    await browser.waitUntil(async () => (await browser.getTitle()) === "hello.pdf — Lamina PDF", {
      timeout: 15_000,
      timeoutMsg: "hello.pdf wasn't saved",
    });
    expect(await subtypes(path)).toContain("Stamp");
  });
});
