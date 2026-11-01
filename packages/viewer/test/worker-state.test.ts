// The document worker keeps no state per render: the render module holds no collection at module
// level that a render could add to. Its old `renderLog` grew with every render and was never
// trimmed; the RPC tests' own worker (test/browser/test-worker.ts) keeps such a log instead. The
// source is read through Vite (`?raw`), so the check needs no Node API.
import { describe, expect, it } from "vitest";
import source from "../src/worker/render.ts?raw";

const COLLECTION = /^(?:export )?(?:const|let|var) \w+(?:: [^=]+)? = (?:\[|new (?:Array|Map|Set|WeakMap)\b)/m;

describe("the document worker's render module", () => {
  it("keeps no collection at module level", () => {
    expect(source.match(COLLECTION)?.[0]).toBeUndefined();
  });

  it("would catch the old render log", () => {
    expect(COLLECTION.test("export const renderLog: number[] = [];")).toBe(true);
  });
});
