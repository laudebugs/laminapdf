// Scenario 8: the 1 GB fixture's first page is painted within 1.5 s of the open, measured in the
// app. CI runners are slower and noisier than the reference machine, so the test asserts twice
// the target and reports the raw time.
import { expect } from "@wdio/globals";
import { fixture, open, paintsSince, waitForSharpSince } from "../app.ts";

/** The target (desktop plan §4) with CI's headroom. */
const LIMIT_MS = 3_000;

describe("the 1 GB document", () => {
  it("should paint its first page quickly", async () => {
    const since = await open(fixture("target", "fixtures", "1gb.pdf"));
    await waitForSharpSince(0, since, 30_000);
    const first = (await paintsSince(since)).find((p) => p.page === 0 && (p.state === "low" || p.state === "sharp"));
    const ms = Math.round((first?.at ?? Number.POSITIVE_INFINITY) - since);
    console.log(`1 GB fixture: first page painted ${ms} ms after Choose a file`);
    expect(ms).toBeLessThan(LIMIT_MS);
  });
});
