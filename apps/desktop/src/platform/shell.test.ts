// The desktop platform's parts that go through the main process (recent files, watching) and the
// window's file-change events, over Tauri's mocked IPC and events: the real `tauriShell()`, the
// main process's answers and events as src-tauri sends them.

import type { RecentFile } from "@laminapdf/ui";
import { fakePlatform } from "@laminapdf/ui/testing";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import type { DesktopDocument } from "../engine/desktop-engine-client.ts";
import { listenForFileChanges, MISSING_EVENT, RELOADED_EVENT, UNREADABLE_EVENT } from "../engine/file-changes.ts";
import { desktopFile } from "../engine/open.ts";
import { tauriShell } from "../engine/tauri-shell.ts";
import { createDesktopPlatform } from "./desktop.ts";

const DOC: DesktopDocument = {
  doc: 7,
  name: "Relevé.pdf",
  size: 1234,
  pageCount: 2,
  encrypted: false,
  needsPassword: false,
  repaired: false,
  pages: [
    { width: 612, height: 792, rotate: 0 },
    { width: 612, height: 792, rotate: 0 },
  ],
};

const ENTRY: RecentFile = { id: "r1", name: "Relevé.pdf", size: 1234, lastOpened: 1, hasHandle: true, missing: false };

/** The commands the webview sent, with their arguments. */
let calls: [string, unknown][] = [];
/** What a command answers; a thrown value is its error. */
let answers: Record<string, (args: unknown) => unknown> = {};

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeAll(() => {
  // Tauri's mocks install themselves on `window`, which Node lacks.
  Object.assign(globalThis, { window: globalThis });
});

beforeEach(() => {
  calls = [];
  answers = {};
  mockConvertFileSrc("macos");
  mockWindows("main");
  mockIPC(
    (command, args) => {
      calls.push([command, args]);
      return answers[command]?.(args);
    },
    { shouldMockEvents: true },
  );
});

afterEach(() => clearMocks());

const desktop = () => createDesktopPlatform(fakePlatform(), tauriShell());

describe("the desktop's recent files", () => {
  it("lists what the main process keeps", async () => {
    answers.recent_list = () => [ENTRY];
    expect(await desktop().recent.list()).toEqual([ENTRY]);
  });

  it("reopens an entry by its id, never a path", async () => {
    answers.recent_open = () => DOC;
    const file = await desktop().recent.reopen?.("r1");
    expect([file?.name, calls]).toEqual(["Relevé.pdf", [["recent_open", { id: "r1" }]]]);
  });

  it("rejects reopening an entry whose file is gone", async () => {
    answers.recent_open = () => {
      throw Object.assign(new Error("not found"), { kind: "io" });
    };
    await expect(desktop().recent.reopen?.("r1")).rejects.toMatchObject({ code: "io" });
  });

  it("hands a file that isn't a PDF to the reader, which shows why", async () => {
    answers.recent_open = () => {
      throw Object.assign(new Error("not a PDF"), { kind: "invalid" });
    };
    const file = await desktop().recent.reopen?.("r1");
    await expect(file?.read(0, 1)).rejects.toMatchObject({ code: "invalid" });
  });

  it("removes and clears through the main process", async () => {
    const recent = desktop().recent;
    await recent.remove("r1");
    await recent.clear();
    expect(calls).toEqual([
      ["recent_remove", { id: "r1" }],
      ["recent_clear", {}],
    ]);
  });
});

describe("watching a file", () => {
  it("runs onChange when the file's document reloads", async () => {
    const file = desktopFile(DOC);
    let changes = 0;
    desktop().watch?.(file, () => changes++);
    await settle();
    await emit(RELOADED_EVENT, { previous: DOC.doc, document: { ...DOC, doc: 8 } });
    await emit(RELOADED_EVENT, { previous: 99, document: { ...DOC, doc: 100 } });
    expect(changes).toBe(1);
  });

  it("stops when unsubscribed", async () => {
    const file = desktopFile(DOC);
    let changes = 0;
    const stop = desktop().watch?.(file, () => changes++);
    await settle();
    stop?.();
    await emit(RELOADED_EVENT, { previous: DOC.doc, document: { ...DOC, doc: 8 } });
    expect(changes).toBe(0);
  });
});

describe("listenForFileChanges", () => {
  it("passes each event on with the document it names", async () => {
    const heard: unknown[] = [];
    await listenForFileChanges(tauriShell(), {
      reloaded: (previous, document) => heard.push(["reloaded", previous, document.doc]),
      missing: (doc) => heard.push(["missing", doc]),
      unreadable: (doc) => heard.push(["unreadable", doc]),
    });
    await emit(RELOADED_EVENT, { previous: 7, document: { ...DOC, doc: 8 } });
    await emit(MISSING_EVENT, { doc: 8 });
    await emit(UNREADABLE_EVENT, { doc: 8 });
    expect(heard).toEqual([
      ["reloaded", 7, 8],
      ["missing", 8],
      ["unreadable", 8],
    ]);
  });
});
