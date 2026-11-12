// Scenario 8: the 1 GB fixture opens and its first page is drawn. The time from Choose a file to
// the first pixels, on the app's clock, is reported against the 1.5 s target, never asserted:
// ADR 0018 keeps wall-clock checks out of the PR lanes.
import { fixture, open, paintsSince, waitForSharpSince } from "../app.ts";

describe("the 1 GB document", () => {
  it("should open and draw its first page", async () => {
    const since = await open(fixture("target", "fixtures", "1gb.pdf"));
    await waitForSharpSince(0, since, 30_000);
    const first = (await paintsSince(since)).find((p) => p.page === 0 && (p.state === "low" || p.state === "sharp"));
    console.log(
      `1 GB fixture: first page painted ${Math.round((first?.at ?? since) - since)} ms after Choose a file (target 1,500 ms)`,
    );
  });
});
