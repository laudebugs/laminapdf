// The docs' "Show a PDF in a web page" example in Chromium, Firefox and WebKit: a plain file opens
// and paints, an encrypted one asks for its password, again after a wrong one, then opens, and a
// cancel keeps the file shown before.
import { afterEach, describe, expect, it } from "vitest";
import { type PdfView, showFile, showPosition, startViewer } from "../../examples/show-a-pdf.js";
import { ENCRYPTED, PASSWORD, read } from "./helpers.js";

const host = document.createElement("div");
Object.assign(host.style, { width: "800px", height: "600px", position: "fixed", top: "0", left: "0" });
document.body.append(host);

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
async function until(done: () => boolean, limit = 600): Promise<void> {
  for (let i = 0; i < limit && !done(); i++) await frame();
  expect(done()).toBe(true);
}

const views: PdfView[] = [];
afterEach(() => {
  for (const view of views.splice(0)) view[Symbol.dispose]();
});

async function started(): Promise<PdfView> {
  const view = await startViewer(host);
  views.push(view);
  return view;
}

/** The fixture as a picked file would come: a `File`. */
async function file(path: string): Promise<Blob> {
  const bytes = await read(path);
  return new File([new Uint8Array(bytes)], path, { type: "application/pdf" });
}

const HELLO = "fixtures/golden/examples/hello.pdf";
const never = async () => {
  throw new Error("asked for a password");
};

describe("show-a-pdf example", () => {
  it("shows the file, paints its page sharp and keeps the position label up to date", async () => {
    const view = await started();
    const label = document.createElement("span");
    const stop = showPosition(view.viewer, label);
    expect(await showFile(view, await file(HELLO), never)).toBe(true);
    expect(view.viewer.getState().pageCount).toBe(1);
    await until(() => view.viewer.stats().pages[0] === "sharp");
    await until(() => label.textContent?.startsWith("Page 1 of 1 · ") === true);
    stop();
  });

  it("asks for the password, again after a wrong one, then opens", async () => {
    const view = await started();
    const asked: boolean[] = [];
    const ask = async (wrong: boolean) => {
      asked.push(wrong);
      return asked.length === 1 ? "not it" : PASSWORD;
    };
    expect(await showFile(view, await file(ENCRYPTED), ask)).toBe(true);
    expect(asked).toEqual([false, true]);
    expect(view.viewer.getState().pageCount).toBeGreaterThan(0);
  });

  it("keeps the file shown when the reader cancels the password", async () => {
    const view = await started();
    await showFile(view, await file(HELLO), never);
    expect(await showFile(view, await file(ENCRYPTED), async () => undefined)).toBe(false);
    expect(view.client.document?.pageCount).toBe(1);
  });
});
