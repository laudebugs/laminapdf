import { describe, expect, it } from "vitest";
import { InterchangeController } from "./interchange.ts";

/** A controller over a document with no comments, which only needs its dialogs' state. */
function controller() {
  return new InterchangeController({
    engine: {} as never,
    viewer: { getState: () => ({ currentPage: 0 }) } as never,
    platform: {} as never,
    toasts: {} as never,
    flow: () => ({}) as never,
    fileName: () => "a.pdf",
    threads: async () => [],
    shown: () => undefined,
    selected: () => [],
  });
}

describe("opening a dialog", () => {
  it("opens the summary to print when Print comment summary asks, and to create otherwise", async () => {
    const c = controller();
    await c.open("summary", { printFirst: true });
    expect(c.getState()).toMatchObject({ dialog: "summary", printFirst: true });
    await c.open("summary");
    expect(c.getState().printFirst).toBe(false);
  });

  it("opens Export on the format asked for", async () => {
    const c = controller();
    await c.open("export", { format: "markdown" });
    expect(c.getState()).toMatchObject({ dialog: "export", start: "markdown" });
  });
});
