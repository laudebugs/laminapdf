// Scenario 5: an encrypted file asks for its password, refuses a wrong one, opens with the right
// one, and after its engine is killed draws again with no second prompt.
import { $, browser, expect } from "@wdio/globals";
import { fixture, killEngine, now, open, waitForSharpSince } from "../app.ts";

describe("an encrypted document", () => {
  it("should ask again after a wrong password and open with the right one", async () => {
    const since = await open(fixture("fixtures", "encrypted", "r3-rc4-128-user.pdf"));
    const field = $('input[aria-label="Password"]');
    await field.setValue("not-the-password");
    await $("button=Open").click();
    await expect($("*=That password didn't open the file")).toBeDisplayed();
    await field.setValue("user-pw");
    await $("button=Open").click();
    await waitForSharpSince(0, since);
  });

  it("should draw again after its engine is killed, without asking for the password", async () => {
    const since = await now();
    await killEngine();
    await expect($("*=Document process restarted")).toBeDisplayed();
    await waitForSharpSince(0, since);
    await expect($('input[aria-label="Password"]')).not.toBeDisplayed();
    await browser.keys("Escape");
  });
});
