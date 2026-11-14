// The last session, fourth run (D4d): relaunched with "Remember recent files" off, after a file was
// open, the app starts with one empty window and keeps no session file; the switch then goes back
// on, as the other runs expect.
import { existsSync } from "node:fs";
import { $, browser, expect } from "@wdio/globals";
import { tabNames, toggleSetting } from "../app.ts";
import { sessionFile } from "../paths.ts";

describe("a relaunch with Remember recent files off", () => {
  it("should start with one empty window and no session file", async () => {
    await $("button=Choose a file").waitForExist();
    expect([(await browser.getWindowHandles()).length, await tabNames(), existsSync(sessionFile)]).toEqual([
      1,
      [],
      false,
    ]);
    await toggleSetting("Remember recent files");
  });
});
