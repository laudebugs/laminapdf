// The docs' "Show a PDF in a web page" example in Chromium, Firefox and WebKit: a plain file opens
// and paints, an encrypted one asks for its password, again after a wrong or empty one, then
// opens, asks again once a restarted worker has lost it, and a cancel keeps the file shown before.
import { afterEach, describe, expect, it } from "vitest";
import { PdfView, showPosition } from "../../examples/show-a-pdf.js";
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
  for (const view of views.splice(0)) view.close();
});

async function started(): Promise<PdfView> {
  const view = await PdfView.start(host);
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
    expect(label.textContent).toBe("");
    expect(await view.show(await file(HELLO), never)).toBe(true);
    expect(view.viewer.getState().pageCount).toBe(1);
    await until(() => view.viewer.stats().pages[0] === "sharp");
    await until(() => label.textContent?.startsWith("Page 1 of 1 · ") === true);
    stop();
  });

  it("asks for the password, again after a wrong or an empty one, then opens", async () => {
    const view = await started();
    const answers = ["not it", "", PASSWORD];
    const asked: boolean[] = [];
    const ask = async (wrong: boolean) => {
      asked.push(wrong);
      return answers[asked.length - 1];
    };
    expect(await view.show(await file(ENCRYPTED), ask)).toBe(true);
    expect(asked).toEqual([false, true, true]);
    expect(view.viewer.getState().pageCount).toBeGreaterThan(0);
  });

  it("asks for the password again after the worker restarts, and shows the file again", async () => {
    const view = await started();
    let asked = 0;
    const ask = async () => {
      asked++;
      return PASSWORD;
    };
    await view.show(await file(ENCRYPTED), ask);
    await view.client.stop();
    await until(() => asked === 2 && view.client.document !== undefined);
    await until(() => view.viewer.stats().pages[0] === "sharp");
  });

  it("keeps the file shown when the reader cancels the password", async () => {
    const view = await started();
    await view.show(await file(HELLO), never);
    const shown = view.client.document;
    expect(await view.show(await file(ENCRYPTED), async () => undefined)).toBe(false);
    expect(view.viewer.getState().pageCount).toBe(1);
    expect(view.client.document).toBe(shown);
    // The worker still has the file: its page draws again.
    view.viewer.invalidate([0]);
    await until(() => view.viewer.stats().pages[0] === "sharp");
  });
});
