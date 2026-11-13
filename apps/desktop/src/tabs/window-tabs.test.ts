// This window's tabs over the real reader controller, opener and engine client, with a shell that
// records the main process's commands and a viewer that records what it shows: opening adds tabs,
// showing another sets the first aside with its view, closing closes its document only, the
// reader's own Close closes the shown tab and shows the next, and a move hands the document over.
// The last session's tabs come back unopened, the shown one opening at once, the others when shown,
// a gone file offering Locate…, and the tabs and views are reported to the main process.

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
  const answer = (command: string, args?: Record<string, unknown>): unknown => {
    if (command === "move_tab") return "doc-1";
    if (command === "unlock_document") return doc(Number(args?.doc), "locked.pdf");
    if (command === "session_open") {
      const token = String(args?.token);
      if (token === "gone") throw new DesktopEngineError("io", "the file could not be opened");
      return doc(100 + Number(token.slice(1)), `${token}.pdf`);
    }
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
    subscribe: () => () => undefined,
  } as unknown as Viewer;
  const tabs = new WindowTabs(shell, client, viewer);
  const reader = new ReaderController({} as Platform, desktopOpener(shell, client, viewer, tabs));
  tabs.attach(reader);
  const scrollTo = (page: number) => {
    view = at(page);
  };
  const closed = () => invoked.filter((c) => c.command === "close_document").map((c) => c.args?.doc);
  return { tabs, reader, client, invoked, restored, scrollTo, closed, gate };
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

describe("the last session's tabs", () => {
  const restoredWindow = (active: number | null) => ({
    tabs: [
      { token: "t1", name: "a.pdf", view: at(4), missing: false },
      { token: "t2", name: "b.pdf", view: at(6), missing: false },
    ],
    active,
  });
  const opened = (invoked: { command: string; args?: Record<string, unknown> }[]) =>
    invoked.filter((c) => c.command === "session_open").map((c) => c.args?.token);

  it("come back unopened, and the shown one opens at once at its view", async () => {
    const { tabs, invoked, restored } = harness();
    await tabs.restore(restoredWindow(1));
    expect(names(tabs)).toEqual(["a.pdf", "b.pdf"]);
    expect(opened(invoked)).toEqual(["t2"]);
    expect([tabs.shown?.file.name, restored.at(-1)]).toEqual(["b.pdf", 6]);
  });

  it("open the others when they are first shown", async () => {
    const { tabs, invoked, client } = harness();
    await tabs.restore(restoredWindow(0));
    await tabs.activate(tabs.getState().tabs[1]?.id ?? "");
    expect(opened(invoked)).toEqual(["t1", "t2"]);
    expect(client.current?.doc).toBe(102);
  });

  it("show a tab whose file is gone on its own tab, unopened, and let Locate… show the file chosen", async () => {
    const { tabs, reader } = harness();
    await tabs.restore({ tabs: [{ token: "gone", name: "gone.pdf", view: null, missing: true }], active: 0 });
    const state = reader.getState();
    const file = state.phase === "error" ? state.file : undefined;
    expect([state.phase, tabs.shown?.file.name, file === undefined ? false : tabs.isUnopened(file)]).toEqual([
      "error",
      "gone.pdf",
      true,
    ]);
    await tabs.located(tabs.shown?.id ?? "", doc(7, "found.pdf"));
    expect([reader.getState().phase, tabs.tabOf(7)?.file.name]).toEqual(["open", "gone.pdf"]);
  });

  it("close a tab that never opened without closing a document", async () => {
    const { tabs, closed } = harness();
    await tabs.restore(restoredWindow(0));
    await tabs.close(tabs.getState().tabs[1]?.id ?? "");
    expect([names(tabs), closed()]).toEqual([["a.pdf"], []]);
  });

  it("are reported to the main process with their documents, tokens, views and the shown one", async () => {
    const { tabs, invoked, scrollTo } = harness();
    await tabs.restore(restoredWindow(0));
    scrollTo(9);
    await new Promise((resolve) => setTimeout(resolve, 600));
    type Reported = { doc: number | null; token: string | null; view: ViewPosition };
    const report = invoked.filter((c) => c.command === "session_tabs").at(-1)?.args as
      | { active: number | null; tabs: Reported[] }
      | undefined;
    const reported = report?.tabs ?? [];
    const pages = reported.map((t) => (t.view as unknown as { point: { page: number } }).point.page);
    expect([report?.active, pages, reported[1]]).toEqual([0, [9, 6], { doc: null, token: "t2", view: at(6) }]);
  });
});
