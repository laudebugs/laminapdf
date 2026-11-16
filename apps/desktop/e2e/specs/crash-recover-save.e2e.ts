// After an unexpected quit, first run: three tabs open in the main window, the last one shown,
// kept in session.json. The run ends as every run does, with the service killing the app, which
// leaves its running mark as a crash would. A forced panic (`e2e_crash_main_panic`) ends the app
// the same way, but WebdriverIO then fails the run when it can't close its session with the app.
// The next run (crash-recover) launches and checks the tab choice (Paper S08).
import { browser } from "@wdio/globals";
import { closeAll, openAndWait } from "../app.ts";
import { example, RECOVER_TABS, savedTabs } from "../session.ts";

describe("a session ended by a crash", () => {
  it("should keep three tabs in the session, the last one in front", async () => {
    await closeAll();
    for (const name of RECOVER_TABS) await openAndWait(example(name));
    await browser.waitUntil(() => JSON.stringify(savedTabs()) === JSON.stringify([RECOVER_TABS]), {
      timeout: 15_000,
      timeoutMsg: `the session file holds ${JSON.stringify(savedTabs())}`,
    });
  });
});
