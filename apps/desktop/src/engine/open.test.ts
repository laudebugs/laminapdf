// The OS's open requests in the webview (open.ts's listenForOsOpens) over Tauri's mocked IPC and
// events: the real `tauriShell()`, the main process's events emitted as src-tauri/src/open.rs sends them.

import type { PlatformFile } from "@laminapdf/ui";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import type { DesktopDocument } from "./desktop-engine-client.ts";
import {
  DRAG_ENTER_EVENT,
  DRAG_LEAVE_EVENT,
  type Dragging,
  listenForOsOpens,
  OPEN_EVENT,
  type OsOpenHandlers,
  REFUSED_EVENT,
} from "./open.ts";
import { tauriShell } from "./tauri-shell.ts";

const DOC: DesktopDocument = {
  doc: 7,
  name: "Relevé.pdf",
  size: 1234,
  pageCount: 1,
  encrypted: false,
  needsPassword: false,
  repaired: false,
  pages: [{ width: 612, height: 792, rotate: 0 }],
};

let commands: string[] = [];

/** Handlers that record what the window was asked to do; `open` waits for `release` when given. */
function recorder(release?: Promise<void>) {
  const seen = {
    opened: [] as PlatformFile[],
    unreadable: [] as string[],
    drags: [] as (Dragging | undefined)[],
    refused: [] as (readonly string[])[],
  };
  const handlers: OsOpenHandlers = {
    open: async (file) => {
      seen.opened.push(file);
      await release;
    },
    unreadable: (name) => seen.unreadable.push(name),
    dragging: (drag) => seen.drags.push(drag),
    refused: (names) => seen.refused.push(names),
  };
  return { seen, handlers };
}

/** Lets the mocked events' handlers and the open queue's promises run. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeAll(() => {
  // Tauri's mocks install themselves on `window`, which Node lacks.
  Object.assign(globalThis, { window: globalThis });
});

beforeEach(() => {
  commands = [];
  mockConvertFileSrc("macos");
  mockWindows("main");
  mockIPC(
    (command) => {
      commands.push(command);
      return undefined;
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => clearMocks());

describe("listenForOsOpens", () => {
  it("takes the pending opens once it listens", async () => {
    await listenForOsOpens(tauriShell(), recorder().handlers);
    expect(commands).toEqual(["take_pending_opens"]);
  });

  it("opens the document the main process sends, by its name and never a path", async () => {
    const { seen, handlers } = recorder();
    await listenForOsOpens(tauriShell(), handlers);
    await emit(OPEN_EVENT, { name: DOC.name, document: DOC });
    await settle();
    expect(seen.opened.map((file) => file.name)).toEqual(["Relevé.pdf"]);
  });

  it("reports a file it couldn't read by name, and opens nothing", async () => {
    const { seen, handlers } = recorder();
    await listenForOsOpens(tauriShell(), handlers);
    await emit(OPEN_EVENT, { name: "gone.pdf", error: { kind: "io", message: "not found" } });
    await settle();
    expect([seen.unreadable, seen.opened.length]).toEqual([["gone.pdf"], 0]);
  });

  it("hands a file the engine refused to the reader, which shows why", async () => {
    const { seen, handlers } = recorder();
    await listenForOsOpens(tauriShell(), handlers);
    await emit(OPEN_EVENT, { name: "broken.pdf", error: { kind: "invalid", message: "not a PDF" } });
    await settle();
    await expect(seen.opened[0]?.read(0, 1)).rejects.toMatchObject({ code: "invalid" });
  });

  it("opens requests one at a time, in order", async () => {
    let release = () => {};
    const { seen, handlers } = recorder(new Promise((resolve) => (release = resolve)));
    await listenForOsOpens(tauriShell(), handlers);
    await emit(OPEN_EVENT, { name: "first.pdf", document: { ...DOC, name: "first.pdf" } });
    await emit(OPEN_EVENT, { name: "second.pdf", document: { ...DOC, doc: 8, name: "second.pdf" } });
    await settle();
    const whileFirstOpens = seen.opened.map((file) => file.name);
    release();
    await settle();
    expect([whileFirstOpens, seen.opened.map((file) => file.name)]).toEqual([
      ["first.pdf"],
      ["first.pdf", "second.pdf"],
    ]);
  });

  it("follows a drag over the window until it leaves", async () => {
    const { seen, handlers } = recorder();
    await listenForOsOpens(tauriShell(), handlers);
    await emit(DRAG_ENTER_EVENT, { pdfs: 2, others: 1 });
    await emit(DRAG_LEAVE_EVENT, null);
    expect(seen.drags).toEqual([{ pdfs: 2, others: 1 }, undefined]);
  });

  it("names the dropped files that aren't PDFs", async () => {
    const { seen, handlers } = recorder();
    await listenForOsOpens(tauriShell(), handlers);
    await emit(REFUSED_EVENT, ["photo.jpg"]);
    expect(seen.refused).toEqual([["photo.jpg"]]);
  });

  it("stops listening when unsubscribed", async () => {
    const { seen, handlers } = recorder();
    const stop = await listenForOsOpens(tauriShell(), handlers);
    stop();
    await emit(REFUSED_EVENT, ["photo.jpg"]);
    expect(seen.refused).toEqual([]);
  });
});
