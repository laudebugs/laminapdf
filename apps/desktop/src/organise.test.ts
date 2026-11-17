import { describe, expect, it } from "vitest";
import type { DesktopEngineClient } from "./engine/desktop-engine-client.ts";
import { DesktopOrganiseEngine } from "./organise.tsx";

/** A client whose shown document has `ids` as its pages, answering the label ranges with `ranges()`. */
function client(ids: readonly string[], ranges: () => Promise<unknown[]>) {
  const listeners = new Set<() => void>();
  const fake = {
    document: { pages: ids.map((id, index) => ({ index, id, width: 612, height: 792, rotation: 0 })) },
    labelRanges: ranges,
    on: (_event: string, listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
  return {
    client: fake as unknown as DesktopEngineClient,
    edited: () => {
      for (const listener of [...listeners]) listener();
    },
  };
}

const labels = (engine: DesktopOrganiseEngine) => engine.getState().pages.map((p) => p.label);

describe("DesktopOrganiseEngine", () => {
  it("labels the pages from the engine's label ranges", async () => {
    const { client: c } = client(["p1.0", "p2.0"], () => Promise.resolve([{ page: "p1.0", style: "upperRoman" }]));
    const engine = new DesktopOrganiseEngine(c);
    await engine.refresh();
    expect(labels(engine)).toEqual(["I", "II"]);
  });

  it("reads the labels again after an edit", async () => {
    let style = "decimal";
    const { client: c, edited } = client(["p1.0"], () => Promise.resolve([{ page: "p1.0", style, prefix: "A-" }]));
    const engine = new DesktopOrganiseEngine(c);
    const stop = engine.follow();
    await Promise.resolve();
    style = "lowerLetters";
    edited();
    await engine.refresh();
    stop();
    expect(labels(engine)).toEqual(["A-a"]);
  });

  it("drops an older read that answers after a newer one", async () => {
    let answer: (ranges: unknown[]) => void = () => undefined;
    const reads = [
      new Promise<unknown[]>((resolve) => {
        answer = resolve;
      }),
      Promise.resolve([{ page: "p1.0", style: "lowerRoman" }]),
    ];
    const { client: c } = client(["p1.0"], () => reads.shift() ?? Promise.resolve([]));
    const engine = new DesktopOrganiseEngine(c);
    const older = engine.refresh();
    await engine.refresh();
    answer([{ page: "p1.0", style: "upperLetters" }]);
    await older;
    expect(labels(engine)).toEqual(["i"]);
  });

  it("keeps the pages unlabelled when a read of the ranges fails", async () => {
    const { client: c } = client(["p1.0"], () => Promise.reject(new Error("engine restarted")));
    const engine = new DesktopOrganiseEngine(c);
    await engine.refresh();
    expect(engine.getState().pages.map((p) => [p.id, p.label])).toEqual([["p1.0", undefined]]);
  });
});
