// The cold-start scenario, in its own run (wdio.cold-open.conf.ts): the app started with a PDF's
// path opens it once the window is ready, through the queue of requests that came before it.
import { expect } from "@wdio/globals";
import { inkedPixels, waitForDocument } from "../app.ts";

describe("opening from the command line", () => {
  it("should open the file named when the app started", async () => {
    await waitForDocument(2);
    expect(await inkedPixels(0)).toBeGreaterThan(0);
  });
});
