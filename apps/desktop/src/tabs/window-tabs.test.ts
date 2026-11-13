// This window's tabs over the real reader controller, opener and engine client, with a shell that
// records the main process's commands and a viewer that records what it shows: opening adds tabs,
// showing another sets the first aside with its view, closing closes its document only, the
// reader's own Close closes the shown tab and shows the next, and a move hands the document over.

import { type Platform, ReaderController } from "@laminapdf/ui";
import type { Viewer, ViewPosition } from "@laminapdf/viewer";
import { describe, expect, it } from "vitest";
import {
  type DesktopDocument,
  DesktopEngineClient,
  DesktopEngineError,
  type DesktopShell,
} from "../engine/desktop-engine-client.ts";
import { desktopFile, desktopOpener, failedFile } from "../engine/open.ts";
import { WindowTabs } from "./window-tabs.ts";

function doc(id: number, name: string): DesktopDocument {
  return {
    doc: id,
    name,
    size: 100,
    pageCount: 3,
    encrypted: false,
    needsPassword: false,
    repaired: false,
    pages: [1, 2, 3].map(() => ({ width: 612, height: 792, rotate: 0 })),
  };
}

/** A view at `page`, as the viewer reports one. */
function at(page: number): ViewPosition {
  return {
    point: { page, x: 0, y: 0 },
    zoom: { mode: "percent", percent: 150 },
    rotation: 0,
  } as unknown as ViewPosition;
}

function harness() {
  const invoked: { command: string; args?: Record<string, unknown> }[] = [];
  // Commands held until the test lets them answer.
  const held = new Map<string, Promise<void>>();
  const gate = (command: string) => {
    let open = (): void => undefined;
    held.set(
      command,
      new Promise<void>((resolve) => {
        open = resolve;
      }),
    );
    return () => open();
  };
  // Where a drag's end sends its tab: `null`, it stays.
  const drop = { to: "doc-1" as string | null };
  const answer = (command: string, args?: Record<string, unknown>): unknown => {
    if (command === "move_tab") return "doc-1";
    if (command === "tab_drag_end") return drop.to;
    if (command === "unlock_document") return doc(Number(args?.doc), "locked.pdf");
    return undefined;
  };
  const shell: DesktopShell = {
    invoke: async <T>(command: string, args?: Record<string, unknown>) => {
      invoked.push({ command, ...(args === undefined ? {} : { args }) });
      await held.get(command);
      return answer(command, args) as T;
    },
    listen: () => Promise.resolve(() => undefined),
    tileBase: "lamina://localhost/",
    fetch: () => Promise.reject(new Error("no tiles here")),
    decode: () => Promise.reject(new Error("no tiles here")),
  } as unknown as DesktopShell;
  const client = new DesktopEngineClient(shell);
  // The viewer shows one document; its view is whatever was restored last, else page 1.
  let view: ViewPosition | undefined;
  const restored: (number | undefined)[] = [];
  const viewer = {
    open: () => {
      view = at(1);
    },
    restore: (position: ViewPosition) => {
      view = position;
      restored.push((position as unknown as { point: { page: number } }).point.page);
    },
    viewPosition: () => view,
  } as unknown as Viewer;
  const tabs = new WindowTabs(shell, client, viewer);
  const reader = new ReaderController({} as Platform, desktopOpener(shell, client, viewer, tabs));
  tabs.attach(reader);
  const scrollTo = (page: number) => {
    view = at(page);
  };
  const closed = () => invoked.filter((c) => c.command === "close_document").map((c) => c.args?.doc);
  return { tabs, reader, client, invoked, restored, scrollTo, closed, gate, drop };
}

const names = (tabs: WindowTabs) => tabs.getState().tabs.map((t) => t.file.name);

describe("a window's tabs", () => {
  it("get one tab per document opened, the last one shown", async () => {
    const { tabs, reader, client, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    await reader.open(desktopFile(doc(3, "c.pdf")));
    expect(names(tabs)).toEqual(["a.pdf", "b.pdf", "c.pdf"]);
    expect(tabs.shown?.file.name).toBe("c.pdf");
    expect(client.current?.doc).toBe(3);
    // Showing another set the others aside: none of them closed.
    expect(closed()).toEqual([]);
  });

  it("show a tab again where it was", async () => {
    const { tabs, reader, restored, scrollTo } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    scrollTo(7);
    await reader.open(desktopFile(doc(2, "b.pdf")));
    const first = tabs.getState().tabs[0];
    await tabs.activate(first?.id ?? "");
    expect(tabs.shown?.file.name).toBe("a.pdf");
    expect(restored.at(-1)).toBe(7);
  });

  it("show the existing tab when the same document opens again", async () => {
    const { tabs, reader } = harness();
    const a = doc(1, "a.pdf");
    await reader.open(desktopFile(a));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    await reader.open(desktopFile(a));
    expect(names(tabs)).toEqual(["a.pdf", "b.pdf"]);
    expect(tabs.shown?.file.name).toBe("a.pdf");
  });

  it("close a tab not shown, and only its document", async () => {
    const { tabs, reader, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    await tabs.close(tabs.getState().tabs[0]?.id ?? "");
    expect(names(tabs)).toEqual(["b.pdf"]);
    expect(closed()).toEqual([1]);
    expect(reader.getState().phase).toBe("open");
  });

  it("close the shown tab with the reader's Close, show the next, and the start screen after the last", async () => {
    const { tabs, reader, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    reader.close();
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(closed()).toEqual([2]);
    reader.close();
    await expect.poll(() => names(tabs)).toEqual([]);
    expect(closed()).toEqual([2, 1]);
    expect(reader.getState().phase).toBe("empty");
  });

  it("show the tab before again, closing nothing, when an open that failed is dismissed", async () => {
    const { tabs, reader, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(failedFile("broken.pdf", new DesktopEngineError("not-a-pdf", "not a PDF")));
    expect(reader.getState().phase).toBe("error");
    expect(tabs.getState().active).toBeUndefined();
    reader.close();
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(reader.getState().phase).toBe("open");
    expect(closed()).toEqual([]);
  });

  it("close a locked document whose password prompt is cancelled, and show the tab before", async () => {
    const { tabs, reader, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile({ ...doc(2, "locked.pdf"), encrypted: true, needsPassword: true }));
    expect(reader.getState().phase).toBe("needs-password");
    reader.cancel();
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(closed()).toEqual([2]);
    expect(names(tabs)).toEqual(["a.pdf"]);
  });

  it("make no tab for a document whose unlock finishes after the prompt was left", async () => {
    const { tabs, reader, closed, gate } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile({ ...doc(2, "locked.pdf"), encrypted: true, needsPassword: true }));
    const release = gate("unlock_document");
    const submitted = reader.submitPassword("secret");
    reader.cancel();
    release();
    await submitted;
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(names(tabs)).toEqual(["a.pdf"]);
    expect(closed()).toContain(2);
  });

  it("step to the next and previous tab, wrapping round", async () => {
    const { tabs, reader } = harness();
    for (const [id, name] of [
      [1, "a.pdf"],
      [2, "b.pdf"],
      [3, "c.pdf"],
    ] as const) {
      await reader.open(desktopFile(doc(id, name)));
    }
    await tabs.step(1);
    expect(tabs.shown?.file.name).toBe("a.pdf");
    await tabs.step(-1);
    expect(tabs.shown?.file.name).toBe("c.pdf");
  });

  it("hand the shown tab to another window with its view, closing nothing", async () => {
    const { tabs, reader, invoked, scrollTo, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    scrollTo(3);
    await tabs.moveShown({ to: "newWindow" });
    const move = invoked.find((c) => c.command === "move_tab");
    expect(move?.args).toMatchObject({ doc: 2, to: { to: "newWindow" }, view: { point: { page: 3 } } });
    expect(names(tabs)).toEqual(["a.pdf"]);
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(closed()).toEqual([]);
  });

  it("leave the start screen without closing anything when the last tab moves away", async () => {
    const { tabs, reader, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await tabs.moveShown({ to: "newWindow" });
    expect(names(tabs)).toEqual([]);
    expect(reader.getState().phase).toBe("empty");
    expect(closed()).toEqual([]);
  });

  it("hand a dragged tab not shown to the window it was dropped on, with the view it kept", async () => {
    const { tabs, reader, invoked, scrollTo, closed } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    scrollTo(4);
    await reader.open(desktopFile(doc(2, "b.pdf")));
    const first = tabs.getState().tabs[0]?.id ?? "";
    tabs.dragStarted(first);
    await tabs.draggedOut(first, true);
    expect(invoked.find((c) => c.command === "tab_drag_start")?.args).toEqual({ doc: 1 });
    const end = invoked.find((c) => c.command === "tab_drag_end");
    expect(end?.args).toMatchObject({ doc: 1, outside: true, view: { point: { page: 4 } } });
    expect(names(tabs)).toEqual(["b.pdf"]);
    expect(tabs.shown?.file.name).toBe("b.pdf");
    expect(closed()).toEqual([]);
  });

  it("show the next tab when the shown one is dragged to another window", async () => {
    const { tabs, reader, client } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    await tabs.draggedOut(tabs.shown?.id ?? "", false);
    expect(names(tabs)).toEqual(["a.pdf"]);
    await expect.poll(() => tabs.shown?.file.name).toBe("a.pdf");
    expect(client.current?.doc).toBe(1);
  });

  it("keep a dragged tab whose drop left it in this window", async () => {
    const { tabs, reader, drop } = harness();
    drop.to = null;
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    await tabs.draggedOut(tabs.shown?.id ?? "", false);
    expect(names(tabs)).toEqual(["a.pdf", "b.pdf"]);
    expect(tabs.shown?.file.name).toBe("b.pdf");
  });

  it("show a tab moved here at the view it had", async () => {
    const { tabs, restored } = harness();
    await tabs.arrived({ document: doc(5, "moved.pdf"), view: at(9) });
    expect(names(tabs)).toEqual(["moved.pdf"]);
    expect(tabs.shown?.file.name).toBe("moved.pdf");
    expect(restored.at(-1)).toBe(9);
  });

  it("let a tab not shown follow its file when it reloads", async () => {
    const { tabs, reader } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    expect(tabs.reloaded(1, doc(11, "a.pdf"))).toBe(true);
    expect(tabs.tabOf(11)?.file.name).toBe("a.pdf");
    expect(tabs.reloaded(99, doc(12, "x.pdf"))).toBe(false);
  });

  it("drop a tab not shown whose document didn't come back from an engine restart", async () => {
    const { tabs, reader } = harness();
    await reader.open(desktopFile(doc(1, "a.pdf")));
    await reader.open(desktopFile(doc(2, "b.pdf")));
    const lost = tabs.restarted({
      window: "main",
      docs: [
        { doc: 1, state: "closed" },
        { doc: 2, state: "open" },
      ],
    });
    expect(lost).toEqual(["a.pdf"]);
    expect(names(tabs)).toEqual(["b.pdf"]);
  });
});
