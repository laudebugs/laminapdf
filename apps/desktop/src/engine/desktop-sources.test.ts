// The other files an edit takes pages from, over a stand-in for the main process's commands.

import { describe, expect, it } from "vitest";
import { type Call, DesktopSources, sourceIds } from "./desktop-sources.ts";

/** Records the commands and answers `add_source` with an ID named after the source. */
function fake(properties: unknown = null) {
  const calls: { command: string; args: Record<string, unknown> | undefined }[] = [];
  const call: Call = <T>(command: string, args?: Record<string, unknown>) => {
    calls.push({ command, args });
    if (command === "add_source") return Promise.resolve({ id: `id-${String(args?.source)}`, pages: ["p1.0"] } as T);
    return Promise.resolve((command === "document_properties" ? properties : null) as T);
  };
  return { calls, sources: new DesktopSources(call) };
}

describe("sourceIds", () => {
  it("reads the source of a page op and the sources of a merge", () => {
    expect(
      sourceIds([
        { op: "insertPagesFrom", args: { source: "a", pages: [] } },
        { op: "mergeDocuments", args: { sources: [{ source: "b" }, { source: "c" }] } },
        { op: "rotatePage", args: { page: "p1.0" } },
      ]),
    ).toEqual(["a", "b", "c"]);
  });
});

describe("DesktopSources", () => {
  it("names the source documents an op log needs, by the IDs the engine gave", async () => {
    const { sources } = fake();
    await sources.add(1, 10);
    await sources.add(1, 11);
    expect(sources.needed(1, [{ op: "insertPagesFrom", args: { source: "id-11" } }])).toEqual([11]);
    expect(sources.needed(2, [{ op: "insertPagesFrom", args: { source: "id-11" } }])).toEqual([]);
  });

  it("gives each source to the document again", async () => {
    const { sources, calls } = fake();
    await sources.restore(1, [10, 11]);
    expect(calls).toEqual([
      { command: "add_source", args: { doc: 1, source: 10 } },
      { command: "add_source", args: { doc: 1, source: 11 } },
    ]);
  });

  it("closes a file nothing took pages from, and keeps one a document holds", async () => {
    const { sources, calls } = fake();
    await sources.add(1, 10);
    await sources.release(10);
    await sources.release(12);
    expect(calls.filter((c) => c.command === "close_document").map((c) => c.args)).toEqual([{ doc: 12 }]);
  });

  it("closes the files a document held when it closes", async () => {
    const { sources, calls } = fake();
    await sources.add(1, 10);
    await sources.closeAll(1);
    expect(calls.filter((c) => c.command === "close_document").map((c) => c.args)).toEqual([{ doc: 10 }]);
  });

  it("keeps a file a second document still holds", async () => {
    const { sources, calls } = fake();
    await sources.add(1, 10);
    await sources.add(2, 10);
    await sources.closeAll(1);
    expect(calls.some((c) => c.command === "close_document")).toBe(false);
  });

  it("keeps a file that is being given to a document when its dialog lets it go", async () => {
    const { sources, calls } = fake();
    const adding = sources.add(1, 10);
    await sources.release(10);
    await adding;
    expect(calls.some((c) => c.command === "close_document")).toBe(false);
  });

  it("keeps every file of a batch open while they are given one after another", async () => {
    const { sources, calls } = fake();
    const done = sources.reserve([10, 11]);
    await sources.release(10);
    await sources.release(11);
    done();
    await sources.release(11);
    expect(calls.filter((c) => c.command === "close_document").map((c) => c.args)).toEqual([{ doc: 11 }]);
  });

  it("reads a source's title from its metadata, then its properties, and none when both are blank or null", async () => {
    const title = (properties: unknown) => fake(properties).sources.title(1);
    expect(await title({ metadata: { title: "Meta" }, properties: { title: "Props" } })).toBe("Meta");
    expect(await title({ metadata: { title: " " }, properties: { title: "Props" } })).toBe("Props");
    expect(await title({ metadata: { title: null }, properties: { title: null } })).toBeUndefined();
  });
});
