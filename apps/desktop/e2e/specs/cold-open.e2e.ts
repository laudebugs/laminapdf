// The cold-start scenario, in its own run (wdio.cold-open.conf.ts): the app started with a PDF's
// path opens it once the window is ready, through the queue of requests that came before it. The
// time to the first sharp page, on the webview's clock (from its start, which follows the app's
// own start-up: cold-start.ts's figure), is reported, never asserted (ADR 0018).
import { expect } from "@wdio/globals";
import { inkedPixels, paintsSince, waitForDocument } from "../app.ts";

describe("opening from the command line", () => {
  it("should open the file named when the app started", async () => {
    await waitForDocument(2);
    expect(await inkedPixels(0)).toBeGreaterThan(0);
    const sharp = (await paintsSince(0)).find((p) => p.page === 0 && p.state === "sharp");
    const at = sharp === undefined ? "no paint recorded" : `${Math.round(sharp.at)} ms`;
    console.log(`Cold start with a file: first page sharp at ${at} on the webview's clock`);
  });
});
