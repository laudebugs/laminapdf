// Scenario 6: a damaged file opens repaired and says so.
import { expect } from "@wdio/globals";
import { fixture, openAndWait, withText } from "../app.ts";

describe("a damaged document", () => {
  it("should open and show Repaired on open", async () => {
    await openAndWait(fixture("fixtures", "corpus", "hello-broken.pdf"));
    await expect(withText("Repaired on open")).toBeDisplayed();
  });
});
