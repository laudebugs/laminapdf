// Scenario 6: a damaged file opens repaired and says so.
import { $, expect } from "@wdio/globals";
import { fixture, openAndWait, waitForSharp } from "../app.ts";

describe("a damaged document", () => {
  it("should open and show Repaired on open", async () => {
    await openAndWait(fixture("fixtures", "corpus", "hello-broken.pdf"));
    await expect($("*=Repaired on open")).toBeDisplayed();
  });
});
