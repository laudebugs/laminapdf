// The last session, third run (D4d): with "Reopen last session" off the app starts with one empty
// window; with "Remember recent files" off, "Reopen last session" can't be turned on and no
// session file is written, whatever opens. The fourth run (session-empty) relaunches with it off.
import { existsSync } from "node:fs";
import { $, browser, expect } from "@wdio/globals";
import { openAndWait, settingDisabled, tabNames, toggleSetting } from "../app.ts";
import { sessionFile } from "../paths.ts";
import { example } from "../session.ts";

describe("no session", () => {
  it("should start with one empty window when Reopen last session is off", async () => {
    await $("button=Choose a file").waitForExist();
    expect([(await browser.getWindowHandles()).length, await tabNames()]).toEqual([1, []]);
  });

  it("should keep no session file while Remember recent files is off", async () => {
    await toggleSetting("Reopen last session");
    await toggleSetting("Remember recent files");
    expect(await settingDisabled("Reopen last session")).toBe(true);
    await openAndWait(example("hello.pdf"));
    // Longer than the e2e build's save interval (2 s) and the tabs' report (0.5 s).
    await browser.pause(4000);
    expect(existsSync(sessionFile)).toBe(false);
  });
});
