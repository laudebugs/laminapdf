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

/** A flow over a file that claims PDF/UA (or can't say), whose plan holds `warnings` until a claim is removed. */
function claiming(options: { readNames: boolean; removal: "works" | "fails" }) {
  let removed = false;
  const saved: unknown[] = [];
  const edits = {
    getState: () => ({ history: { position: 1, unsaved: 1 }, saving: false }),
    subscribe: () => () => undefined,
    plan: async () => ({
      constraint: "free",
      signatures: [],
      refusal: undefined,
      wouldInvalidate: [],
      conformanceWarnings: removed
        ? []
        : [{ code: "figure-without-alt", message: "a replaced figure has no alternative text" }],
    }),
    apply: async () => {
      if (options.removal === "fails") throw new Error("refused");
      removed = true;
    },
  };
  const engine = {
    view: async () => {
      if (!options.readNames) throw new Error("no metadata");
      return { client: { metadata: async () => ({ pdfua: 1 }) } };
    },
    saveEdits: async (saveOptions: unknown) => {
      saved.push(saveOptions);
      return { name: "a.pdf", downloaded: false, report: {} };
    },
  };
  const saveFlow = new SaveFlow({
    engine: engine as never,
    viewer: { edits } as never,
    platform: {} as never,
    toasts: { add: () => undefined } as never,
    fileName: () => "a.pdf",
  });
  return { saveFlow, saved };
}

/** Starts a save and answers its Check before saving with `answer`; the save's result. */
async function saveAnswering(flow: ReturnType<typeof claiming>, answer: "remove" | "fix" | "close"): Promise<boolean> {
  const result = flow.saveFlow.save("save");
  for (let i = 0; i < 50 && flow.saveFlow.getState().asking === undefined; i++)
    await new Promise((r) => setTimeout(r, 0));
  expect(flow.saveFlow.getState().asking).toMatchObject({ kind: "conformance", saving: true });
  flow.saveFlow.answer(answer);
  return result;
}

describe("Check before saving", () => {
  it("asks even when the file's claims can't be read, and saves nothing on Close", async () => {
    const flow = claiming({ readNames: false, removal: "works" });
    expect(await saveAnswering(flow, "close")).toBe(false);
    expect(flow.saved).toEqual([]);
  });

  it("sends Fix it first to whoever fixes, and saves nothing", async () => {
    const flow = claiming({ readNames: true, removal: "works" });
    let fixed = 0;
    flow.saveFlow.onFix(() => fixed++);
    expect(await saveAnswering(flow, "fix")).toBe(false);
    expect(fixed).toBe(1);
    expect(flow.saved).toEqual([]);
  });

  it("saves nothing when the claim couldn't be removed", async () => {
    const flow = claiming({ readNames: true, removal: "fails" });
    expect(await saveAnswering(flow, "remove")).toBe(false);
    expect(flow.saved).toEqual([]);
  });

  it("saves once the claim is gone from the plan", async () => {
    const flow = claiming({ readNames: true, removal: "works" });
    expect(await saveAnswering(flow, "remove")).toBe(true);
    expect(flow.saved).toHaveLength(1);
  });
});
