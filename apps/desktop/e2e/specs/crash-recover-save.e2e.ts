// After an unexpected quit, first run: three tabs open in the main window, the last one shown,
// kept in session.json, then a forced main-process panic ends the app with its running mark still
// there. The next run (crash-recover) launches and checks the tab choice (Paper S08).
import { browser } from "@wdio/globals";
import { closeAll, invoke, openAndWait } from "../app.ts";
import { example, RECOVER_TABS, savedTabs } from "../session.ts";

describe("a session ended by a crash", () => {
  it("should keep three tabs in the session, then crash with the last one in front", async () => {
    await closeAll();
    for (const name of RECOVER_TABS) await openAndWait(example(name));
    await browser.waitUntil(() => JSON.stringify(savedTabs()) === JSON.stringify([RECOVER_TABS]), {
      timeout: 15_000,
      timeoutMsg: `the session file holds ${JSON.stringify(savedTabs())}`,
    });
    // The app ends here, so the command's answer never comes.
    await invoke("e2e_crash_main_panic").catch(() => undefined);
    // It ended from the panic, not from the service's kill after the run: it stops answering.
    await browser.waitUntil(
      async () => {
        try {
          await browser.execute(() => 1);
          return false;
        } catch {
          return true;
        }
      },
      { timeout: 15_000, timeoutMsg: "the app kept running after the panic" },
    );
  });
});
