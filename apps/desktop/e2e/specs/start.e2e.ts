// Scenario 1: the app starts and shows its empty state.
import { $, expect } from "@wdio/globals";

describe("the desktop reader", () => {
  it("should show the empty state when it starts", async () => {
    await expect($("button=Choose a file")).toBeDisplayed();
  });
});
