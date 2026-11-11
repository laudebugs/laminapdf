import { describe, expect, it } from "vitest";
import { ReaderController } from "../src/state/reader-controller.ts";
import { transition } from "../src/state/reader-machine.ts";
import { handTool, type InteractionMode, type ToolHost, type ToolModule, ToolRegistry } from "../src/tools/tools.ts";
import { fakeFile, fakePlatform } from "./fakes.ts";

// A later round's tool, added by declaration merging as the README shows.
declare module "../src/tools/tools.ts" {
  interface InteractionModes {
    measure: true;
  }
}

const MEASURE: ToolModule = { name: "measure", cursor: "crosshair", layer: "annot", selectsText: false };

describe("the interaction modes", () => {
  it("start with select, hand and text select, select on", () => {
    const tools = new ToolRegistry();
    expect([[...tools.getState().tools.keys()], tools.getState().current]).toEqual([
      ["select", "hand", "textSelect"],
      "select",
    ]);
  });

  it("take a tool a later round registers, by its merged name", () => {
    const tools = new ToolRegistry();
    tools.register(MEASURE);
    const name: InteractionMode = "measure";
    expect([tools.use(name), tools.current()?.cursor]).toEqual([true, "crosshair"]);
  });

  it("ignore a name nobody registered", () => {
    const tools = new ToolRegistry();
    expect([tools.use("measure"), tools.getState().current]).toEqual([false, "select"]);
  });

  it("refuse a tool registered twice", () => {
    const tools = new ToolRegistry();
    expect(() => tools.register({ ...MEASURE, name: "hand" })).toThrow(/already registered/);
  });

  it("go back to select when the tool on is removed", () => {
    const tools = new ToolRegistry();
    const remove = tools.register(MEASURE);
    tools.use("measure");
    remove();
    expect(tools.getState().current).toBe("select");
  });

  it("tell subscribers when the tool changes", () => {
    const tools = new ToolRegistry();
    const heard: string[] = [];
    tools.subscribe(() => heard.push(tools.getState().current));
    tools.use("hand");
    tools.use("hand");
    expect(heard).toEqual(["hand"]);
  });

  it("scroll the page under a hand's drag", () => {
    const hand = handTool();
    const moved: number[][] = [];
    const host = { element: {} as HTMLElement, scrollBy: (dx: number, dy: number) => moved.push([dx, dy]) } as ToolHost;
    const at = (x: number, y: number, button = 0) => ({ clientX: x, clientY: y, button }) as PointerEvent;
    const took = hand.pointerDown?.(at(100, 100), host);
    hand.pointerMove?.(at(90, 70), host);
    hand.pointerUp?.(at(90, 70), host);
    hand.pointerMove?.(at(0, 0), host);
    expect([took, moved]).toEqual([true, [[10, 30]]]);
  });

  it("leave a hand's other buttons alone", () => {
    expect(handTool().pointerDown?.({ button: 2 } as PointerEvent, {} as ToolHost)).toBe(false);
  });
});

describe("the reader machine's modes", () => {
  it("takes any interaction mode while a document is open", () => {
    const open = { phase: "open", file: fakeFile("a.pdf"), pageCount: 1, mode: "select" } as const;
    expect(transition(open, { type: "set-mode", mode: "hand" })).toMatchObject({ mode: "hand" });
  });
});

describe("the close guard", () => {
  async function opened(guard: () => Promise<boolean>) {
    const reader = new ReaderController(fakePlatform(), async () => ({ pageCount: 1 }));
    await reader.open(fakeFile("a.pdf"));
    reader.setCloseGuard(guard);
    return reader;
  }

  it("keeps the document when the guard says no", async () => {
    const reader = await opened(async () => false);
    expect([await reader.requestClose(), reader.getState().phase]).toEqual([false, "open"]);
  });

  it("closes when the guard says yes", async () => {
    const reader = await opened(async () => true);
    expect([await reader.requestClose(), reader.getState().phase]).toEqual([true, "empty"]);
  });

  it("asks before another file replaces the open one", async () => {
    let asked = 0;
    const reader = await opened(async () => {
      asked += 1;
      return false;
    });
    await reader.open(fakeFile("b.pdf"));
    expect([asked, reader.getState()]).toMatchObject([1, { phase: "open", file: { name: "a.pdf" } }]);
  });

  it("doesn't ask when the shell reopens with discard", async () => {
    let asked = 0;
    const reader = await opened(async () => {
      asked += 1;
      return true;
    });
    await reader.open(fakeFile("a.pdf"), { discard: true });
    expect(asked).toBe(0);
  });

  it("asks nothing with no document open", async () => {
    const reader = new ReaderController(fakePlatform(), async () => ({ pageCount: 1 }));
    let asked = 0;
    reader.setCloseGuard(async () => {
      asked += 1;
      return false;
    });
    await reader.open(fakeFile("a.pdf"));
    expect([asked, reader.getState().phase]).toEqual([0, "open"]);
  });
});
