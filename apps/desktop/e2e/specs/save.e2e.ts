// Saving edits (E1 plan §7): File ▸ Save appends the edit to the file as a revision, which the
// app and pdf.js both read back, and the app's own write never reloads the document. Another
// app's change to a file with unsaved edits asks first: Reload takes the other app's version,
// Overwrite keeps the edits. Save as writes a new file that the document reads from then on, and a
// write that fails (a read-only file) leaves the edits unsaved and the file as it was. The scenario
// changes the file from the test process, as another app would.
import { chmodSync, copyFileSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import { fixture, focusWindow, invoke, menuClick, openAndWait, view, withText } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "save");
const saved = join(dir, "saved.pdf");
const savedAs = join(dir, "saved as.pdf");
const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** Page 1's `/Rotate` as the engine reports it. */
async function rotation(): Promise<number | undefined> {
  return browser.execute(() => globalThis.__laminaE2e?.rotation(0));
}

/** The pages of the file at `path` as pdf.js reads them: how many, and page 1's rotation. */
async function readByPdfJs(path: string): Promise<{ pages: number; rotate: number }> {
  const task = getDocument({ data: new Uint8Array(readFileSync(path)), verbosity: 0 });
  const read = await task.promise;
  try {
    return { pages: read.numPages, rotate: (await read.getPage(1)).rotate };
  } finally {
    await task.destroy();
  }
}

/** Turns the shown page a quarter right and waits for the title's "•". */
async function turn(file: string, to: number): Promise<void> {
  await menuClick("cmd:pages.rotate-right");
  await browser.waitUntil(
    async () => (await rotation()) === to && (await browser.getTitle()) === `• ${file} — Lamina PDF`,
    { timeoutMsg: `the page didn't turn to ${to}° with "${file}" unsaved in the title` },
  );
}

/** Waits until the title says `file` holds no unsaved edits. */
async function waitForSaved(file: string): Promise<void> {
  await browser.waitUntil(async () => (await browser.getTitle()) === `${file} — Lamina PDF`, {
    timeout: 15_000,
    timeoutMsg: `the title didn't say ${file} is saved`,
  });
}

/** Waits for the question about a file another app changed, and answers it with `button`. */
async function answerConflict(file: string, button: "Reload" | "Overwrite"): Promise<void> {
  const question = $("[role=alertdialog]");
  await question.waitForDisplayed({ timeout: 15_000, timeoutMsg: "the outside change didn't ask" });
  expect(await question.getText()).toContain(`${file} changed on disk`);
  await $(`button=${button}`).click();
  await question.waitForDisplayed({ reverse: true });
}

describe("saving edits in place", () => {
  before(() => {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    copyFileSync(example("hello.pdf"), saved);
  });

  it("saves an edit in place as a revision appended to the file, which reopens with it", async () => {
    const before = readFileSync(saved);
    await openAndWait(saved);
    await focusWindow();
    await turn("saved.pdf", 90);
    await menuClick("cmd:file.save");
    await waitForSaved("saved.pdf");
    await expect(withText("Saved saved.pdf")).toExist();
    const after = readFileSync(saved);
    // An incremental save only adds to the file: the bytes before it are the file as it was.
    expect([after.length > before.length, after.subarray(0, before.length).equals(before)]).toEqual([true, true]);
    expect(await readByPdfJs(saved)).toEqual({ pages: 1, rotate: 90 });
    // The app's own write is no change from outside: nothing reloads.
    await browser.pause(2_000);
    await expect(withText("Reloaded: saved.pdf changed on disk")).not.toExist();
    await menuClick("cmd:tab.close");
    await $("button=Choose a file").waitForExist();
    await openAndWait(saved);
    expect(await rotation()).toBe(90);
  });

  it("asks when another app changes a file with unsaved edits, and Reload takes its version", async () => {
    await turn("saved.pdf", 180);
    const invoice = await readByPdfJs(example("invoice.pdf"));
    copyFileSync(example("invoice.pdf"), saved);
    await answerConflict("saved.pdf", "Reload");
    await browser.waitUntil(async () => (await view()).pageCount === invoice.pages, {
      timeout: 15_000,
      timeoutMsg: "Reload didn't show the other app's version",
    });
    await waitForSaved("saved.pdf");
    expect(await rotation()).toBe(invoice.rotate);
  });

  it("overwrites another app's change with the edits when told to", async () => {
    await turn("saved.pdf", 90);
    copyFileSync(example("hello.pdf"), saved);
    await answerConflict("saved.pdf", "Overwrite");
    await waitForSaved("saved.pdf");
    // The other app's one-page file is gone: the file is the edited invoice, written whole.
    const invoice = await readByPdfJs(example("invoice.pdf"));
    expect(await readByPdfJs(saved)).toEqual({ pages: invoice.pages, rotate: 90 });
  });
});

describe("saving edits as a new file", () => {
  after(() => {
    chmodSync(savedAs, 0o644);
  });

  it("saves as a new file, which the document reads and saves to from then on", async () => {
    const original = readFileSync(saved);
    await turn("saved.pdf", 180);
    await invoke("e2e_queue_save_as", { path: savedAs });
    await menuClick("cmd:file.save-as");
    await waitForSaved("saved as.pdf");
    expect((await readByPdfJs(savedAs)).rotate).toBe(180);
    expect(readFileSync(saved).equals(original)).toBe(true);
    await turn("saved as.pdf", 270);
    await menuClick("cmd:file.save");
    await waitForSaved("saved as.pdf");
    expect((await readByPdfJs(savedAs)).rotate).toBe(270);
  });

  it("keeps the edits unsaved and the file as it was when the file can't be written", async () => {
    chmodSync(savedAs, 0o444);
    const before = readFileSync(savedAs);
    await turn("saved as.pdf", 0);
    await menuClick("cmd:file.save");
    await withText("Couldn't save saved as.pdf: permission to write it was refused").waitForExist({
      timeout: 15_000,
    });
    expect(await browser.getTitle()).toBe("• saved as.pdf — Lamina PDF");
    expect(readFileSync(savedAs).equals(before)).toBe(true);
    // Undone, the document matches its file again, so the next scenario closes it without asking.
    await menuClick("cmd:edit.undo");
    await waitForSaved("saved as.pdf");
  });
});
