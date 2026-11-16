// After an unexpected quit, second run: the app relaunched after crash-recover-save shows S08's
// tab choice before any tab reopens, with the tab that was in front unchecked; Reopen brings back
// the others only.
import { $, browser, expect } from "@wdio/globals";
import { tabNames } from "../app.ts";
import { RECOVER_TABS } from "../session.ts";

const FRONT = RECOVER_TABS.at(-1) ?? "";
const OTHERS = RECOVER_TABS.slice(0, -1);

/** The choice's check box for the tab named `name`. */
const tabBox = (name: string) =>
  $(`//*[@role="dialog"]//label[contains(., ${JSON.stringify(name)})]//input[@type="checkbox"]`);

describe("the tabs after a crash", () => {
  it("should ask which tabs to reopen before any reopens", async () => {
    await expect($('//*[@role="dialog"][.//*[contains(text(), "Lamina PDF quit unexpectedly")]]')).toBeDisplayed();
    expect(await tabNames()).toEqual([]);
  });

  it("should leave the tab that was in front unchecked", async () => {
    const checked = await Promise.all(RECOVER_TABS.map((name) => tabBox(name).isSelected()));
    expect(checked).toEqual(RECOVER_TABS.map((name) => name !== FRONT));
  });

  it("should reopen only the checked tabs", async () => {
    await $(`button=Reopen ${OTHERS.length} tabs`).click();
    await browser.waitUntil(async () => JSON.stringify(await tabNames()) === JSON.stringify(OTHERS), {
      timeout: 15_000,
      timeoutMsg: "the chosen tabs didn't reopen",
    });
    expect(await tabNames()).toEqual(OTHERS);
  });
});
