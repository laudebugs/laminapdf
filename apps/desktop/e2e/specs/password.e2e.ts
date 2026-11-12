// Scenario 5: an encrypted file asks for its password, refuses a wrong one, opens with the right
// one, and after its engine is killed draws again with no second prompt.
import { $, browser, expect } from "@wdio/globals";
import { drawsAfterZoomOut, fixture, killEngine, open, waitForSharpSince, withText } from "../app.ts";

describe("an encrypted document", () => {
  it("should ask again after a wrong password and open with the right one", async () => {
    const since = await open(fixture("fixtures", "encrypted", "r3-rc4-128-user.pdf"));
    const field = $('input[aria-label="Password"]');
    await field.setValue("not-the-password");
    await $("button=Open").click();
    await expect(withText("Passwords are case-sensitive")).toBeDisplayed();
    await field.setValue("user-pw");
    await $("button=Open").click();
    await waitForSharpSince(0, since);
  });

  it("should keep drawing after its engine is killed, without asking for the password", async () => {
    await killEngine();
    await expect(withText("Document process restarted")).toBeDisplayed();
    await drawsAfterZoomOut();
    await expect($('input[aria-label="Password"]')).not.toBeDisplayed();
    await browser.keys("Escape");
  });
});
