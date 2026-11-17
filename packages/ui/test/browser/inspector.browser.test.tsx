// The object inspector in a browser: a read that failed says why and asks again on Try again, and
// a tab's facts are asked for only when the tab is first shown.

import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "vitest/browser";
import { type InspectFactsTab, ObjectInspector, type ObjectInspectorProps } from "../../src/dialogs.ts";
import { cleanup, renderIn, settled } from "./render.tsx";

afterEach(cleanup);

const TRAILER = { type: "dictionary", entries: [["Root", { type: "reference", num: 1, generation: 0 }]] } as const;

/** The inspector with `props` over a summary read and nothing else. */
function inspector(props: Partial<ObjectInspectorProps>): HTMLElement {
  return renderIn(
    <div style={{ blockSize: "600px", display: "flex", flexDirection: "column" }}>
      <ObjectInspector
        summary={{ status: "ready", value: { trailer: TRAILER, version: "1.7", objects: 3, revisions: 1 } }}
        onRetrySummary={() => {}}
        entries={new Map()}
        objects={new Map()}
        onRead={() => {}}
        referrers={new Map()}
        onReadReferrers={() => {}}
        tabs={{}}
        onReadTab={() => {}}
        streams={new Map()}
        onReadStream={() => {}}
        pageOf={() => undefined}
        onBack={() => {}}
        onShowPage={() => {}}
        onCopy={() => {}}
        onSaveRaw={() => {}}
        {...props}
      />
    </div>,
  );
}

describe("ObjectInspector", () => {
  it("says why an object couldn't be read, and reads it again on Try again", async () => {
    const asked: number[] = [];
    const host = inspector({
      goTo: 1,
      objects: new Map([[1, { status: "failed", reason: "the worker stopped" }]]),
      onRead: (num) => asked.push(num),
    });
    await settled();
    expect(host.querySelector("[role=alert]")?.textContent).toContain("the worker stopped");
    const retry = [...host.querySelectorAll("button")].find((b) => b.textContent === "Try again");
    await userEvent.click(retry as HTMLElement);
    expect(asked).toEqual([1]);
  });

  it("says why the summary couldn't be read, and reads it again on Try again", async () => {
    let retried = 0;
    const host = inspector({
      summary: { status: "failed", reason: "data pending" },
      onRetrySummary: () => {
        retried += 1;
      },
    });
    await settled();
    const retry = [...host.querySelectorAll("button")].find((b) => b.textContent === "Try again");
    await userEvent.click(retry as HTMLElement);
    expect(retried).toBe(1);
  });

  it("asks for a tab's facts only when the tab is shown", async () => {
    const asked: InspectFactsTab[] = [];
    const host = inspector({ onReadTab: (tab) => asked.push(tab) });
    await settled();
    expect(asked).toEqual([]);
    const fonts = [...host.querySelectorAll("[role=tab]")].find((t) => t.textContent?.includes("Fonts"));
    await userEvent.click(fonts as HTMLElement);
    await settled();
    expect(asked).toEqual(["fonts"]);
  });
});
