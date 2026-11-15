// The missing-file session, second run (D4f): the app relaunched after session-missing-save, with
// one of its two files deleted, says so in its restore notice, marks that tab, shows it from the
// notice, and asks before a located file of another size takes its place.
import { $, browser, expect } from "@wdio/globals";
import { fixture, queuePick, waitForShown } from "../app.ts";

const NOTICE = "Restored your last session. 2 tabs, each at the page you were reading. 1 file was not found.";

describe("the last session with a missing file", () => {
  it("should say what came back and that one file was not found", async () => {
    const status = $(`//*[@role="status"][contains(., "Restored your last session.")]`);
    await status.waitForExist({ timeout: 20_000, timeoutMsg: "no restore notice" });
    expect(await status.getText()).toBe(NOTICE);
  });

  it("should mark the missing file's tab", async () => {
    const mark = $('//*[@role="tab"][contains(., "vanishing.pdf")]//*[@aria-label="Not found"]');
    await mark.waitForExist({ timeoutMsg: "the missing tab carries no mark" });
  });

  it("should show the missing file's tab from the notice", async () => {
    await $("button=Show missing file").click();
    await waitForShown("vanishing.pdf");
    // The missing-file page's own button: only that page offers Locate….
    await $("button=Locate…").waitForExist({ timeoutMsg: "no missing-file page" });
  });

  it("should ask before a located file of another size opens in its place", async () => {
    // The scripted pick stands in for the dialog: a file of another size than the one saved.
    await queuePick(fixture("fixtures", "golden", "examples", "markdown_to_pdf.pdf"));
    await $("button=Locate…").click();
    const ask = $('[role="alertdialog"]');
    await ask.waitForExist({ timeoutMsg: "no question before the different file opened" });
    await $("button=Open anyway").click();
    await browser.waitUntil(
      async () => !(await $('//*[@role="tab"][contains(., "vanishing.pdf")]//*[@aria-label="Not found"]').isExisting()),
      { timeoutMsg: "the tab is still marked missing" },
    );
    expect(await $('//*[@role="status"][contains(., "Restored your last session.")]').getText()).toContain(
      "2 tabs, each at the page you were reading.",
    );
  });
});
