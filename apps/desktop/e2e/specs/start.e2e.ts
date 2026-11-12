// Scenario 1: the app starts and shows its empty state. Hosted runners can't hold the 300 ms cold
// start (cold-start.ts reports it), so this only guards against a hang: 5 s.
import { $, expect } from "@wdio/globals";

describe("the desktop reader", () => {
  it("should show the empty state when it starts", async () => {
    await expect($("button=Choose a file")).toBeDisplayed({ wait: 5_000 });
  });
});
