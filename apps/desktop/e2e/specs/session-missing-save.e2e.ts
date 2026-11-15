// The missing-file session, first run (D4f): two tabs, one of them a working copy that the next
// run's launch deletes first (wdio.session-missing.conf.ts), kept in the session file.
import { browser, expect } from "@wdio/globals";
import { closeAll, openAndWait } from "../app.ts";
import { example, savedTabs, vanishing } from "../session.ts";

const SAVED = [["vanishing.pdf", "invoice_from_json.pdf"]];

describe("a session with a file that will go missing", () => {
  it("should be kept with both tabs", async () => {
    await closeAll();
    await openAndWait(vanishing);
    await openAndWait(example("invoice_from_json.pdf"));
    await browser.waitUntil(() => JSON.stringify(savedTabs()) === JSON.stringify(SAVED), {
      timeout: 15_000,
      timeoutMsg: `the session file holds ${JSON.stringify(savedTabs())}`,
    });
    expect(savedTabs()).toEqual(SAVED);
  });
});
