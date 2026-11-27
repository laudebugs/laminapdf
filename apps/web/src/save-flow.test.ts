import type { SaveOptions } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { SaveFlow } from "./save-flow.ts";

/** A flow over a fake edit session whose position the test moves, recording each plan asked for. */
function flow() {
  const planned: SaveOptions[] = [];
  const listeners = new Set<() => void>();
  let state = { history: { position: 1, unsaved: 1 }, saving: false };
  const edits = {
    getState: () => state,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    plan: async (options: SaveOptions) => {
      planned.push(options);
      return { constraint: "free", signatures: [], refusal: undefined, wouldInvalidate: [] };
    },
  };
  const saveFlow = new SaveFlow({
    engine: {} as never,
    viewer: { edits } as never,
    platform: {} as never,
    toasts: {} as never,
    fileName: () => "a.pdf",
  });
  const goTo = async (position: number) => {
    state = { history: { position, unsaved: 1 }, saving: false };
    for (const listener of listeners) listener();
    await Promise.resolve();
    await Promise.resolve();
  };
  return { saveFlow, planned, goTo };
}

describe("the rewrite a flatten asks for", () => {
  it("ends when the flatten is undone and comes back when it is redone", async () => {
    const { saveFlow, planned, goTo } = flow();
    saveFlow.start();
    saveFlow.rewriteNextSave(true);
    await goTo(0);
    await goTo(1);
    const modes = planned.map((options) => options.mode);
    // Asked for with the flatten done; plain once it is undone; asked for again once it is redone.
    expect(modes.slice(-3)).toEqual(["fullRewrite", undefined, "fullRewrite"]);
  });
});
