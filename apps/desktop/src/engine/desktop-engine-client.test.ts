// The desktop engine client over Tauri's mocked IPC and events (@tauri-apps/api/mocks): the real
// `tauriShell()`, with only the webview's `fetch` and `createImageBitmap` stood in for, since Node
// has neither the `lamina://` scheme nor a decoder.

import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockConvertFileSrc, mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type DesktopDocument,
  DesktopEngineClient,
  FAILED_EVENT,
  RESTARTED_EVENT,
  type RestartedPayload,
} from "./desktop-engine-client.ts";
import { desktopFile, desktopOpener } from "./open.ts";
import { tauriShell } from "./tauri-shell.ts";
import { type TileAddress, tileUrl } from "./tile-url.ts";
import wireFixture from "./wire.fixture.json" with { type: "json" };

const LETTER = { width: 612, height: 792, rotate: 0 };
const PAGES = [
  { id: "p1.0", ...LETTER },
  { id: "p2.0", ...LETTER },
];

const PLAIN: DesktopDocument = {
  doc: 4,
  name: "letter.pdf",
  size: 1234,
  pageCount: 2,
  encrypted: false,
  needsPassword: false,
  repaired: false,
  pages: PAGES,
};

/** A file picked to take pages from. */
const OTHER: DesktopDocument = { ...PLAIN, doc: 9, name: "other.pdf" };

const LOCKED: DesktopDocument = { ...PLAIN, doc: 5, encrypted: true, needsPassword: true, pageCount: 0, pages: [] };

/** A bitmap that records whether it was closed. */
class FakeBitmap {
  closed = false;
  readonly width: number;
  readonly height: number;
  constructor(width: number, height: number) {
    this.width = width;
    this.height = height;
  }
  close(): void {
    this.closed = true;
  }
}

let commands: { command: string; args: unknown }[] = [];
/** How the next save answers: what it saved, `null` for a cancelled Save as, or a refusal. */
let saveAnswer: unknown;

/** An edit's outcome, as the main process sends it: page 1 turned a quarter. */
const TURNED = {
  changed: true,
  reversal: null,
  touched: ["p1.0"],
  pageOrder: null,
  pages: [{ ...PAGES[0], rotate: 90 }, PAGES[1]],
  createdOutlineItems: [],
  createdAnnotations: [],
  outlineRepaired: false,
};
let bitmaps: FakeBitmap[] = [];

/** The name the main process says a copy with a refused change was saved as; null: the dialog was cancelled. */
let copyName: string | null = "copy.pdf";

/** What the main process answers the commands that always answer the same. */
const ANSWERS: Record<string, unknown> = {
  edit_document: TURNED,
  pick_source: OTHER,
  add_source: { id: "abc", pages: ["p1.0", "p2.0"] },
  plan_save: {
    mode: "incremental",
    constraint: "free",
    signatures: [],
    wouldInvalidate: [],
    conformanceWarnings: [],
    unchanged: false,
  },
};

/** Answers every tile fetch with `answer`'s response, after `delay` ms. */
function serveTiles(answer: (url: URL) => Response, delay = 0): URL[] {
  const asked: URL[] = [];
  vi.stubGlobal("fetch", (url: string, init: { signal?: AbortSignal } = {}) => {
    asked.push(new URL(url));
    return new Promise<Response>((resolve, reject) => {
      const timer = setTimeout(() => resolve(answer(new URL(url))), delay);
      init.signal?.addEventListener("abort", () => {
        clearTimeout(timer);
        reject(new DOMException("aborted", "AbortError"));
      });
    });
  });
  return asked;
}

const tile = () => new Response(new Blob([new Uint8Array(8)], { type: "image/webp" }));

beforeAll(() => {
  // Tauri's mocks install themselves on `window`, which Node lacks.
  Object.assign(globalThis, { window: globalThis });
});

beforeEach(() => {
  copyName = "copy.pdf";
  commands = [];
  bitmaps = [];
  mockConvertFileSrc("macos");
  mockWindows("main");
  mockIPC(
    (command, args) => {
      commands.push({ command, args });
      if (command === "unlock_document") {
        const { password } = args as { password: string };
        if (password !== "secret")
          return Promise.reject({ kind: "wrong-password", message: "the password doesn't open this document" });
        return { ...LOCKED, needsPassword: false, pageCount: 2, pages: PAGES };
      }
      if (command in ANSWERS) return ANSWERS[command];
      if (command === "save_copy_with") return copyName;
      if (command === "save_document" || command === "save_document_as")
        return saveAnswer instanceof Error ? Promise.reject(saveAnswer.cause) : saveAnswer;
      return undefined;
    },
    { shouldMockEvents: true },
  );
  vi.stubGlobal("createImageBitmap", (blob: Blob) => {
    const bitmap = new FakeBitmap(512, 300 + blob.size);
    bitmaps.push(bitmap);
    return Promise.resolve(bitmap);
  });
});

afterEach(() => {
  clearMocks();
  vi.unstubAllGlobals();
});

function client(): DesktopEngineClient {
  return new DesktopEngineClient(tauriShell());
}

/** The viewer's edit session as far as a save goes: the copy, the write, then the adoption. */
function session(engine: DesktopEngineClient) {
  return {
    async save(options: Parameters<DesktopEngineClient["copyEdits"]>[0], write: () => Promise<void>) {
      const copy = await engine.copyEdits(options);
      await write();
      return engine.adoptEdits(copy.token);
    },
  };
}

/** A save that wrote the file as `name`. */
const savedAs = (name: string) => ({ name, mode: "incremental", historyCleared: false, metadataNotUpdated: false });

const CLIP = { x: 512, y: 0, width: 512, height: 300 };

describe("rendering", () => {
  it("fetches the tile over lamina:// as WebP and returns its bitmap", async () => {
    const asked = serveTiles(tile);
    const engine = client();
    engine.show(PLAIN);
    const drawn = await engine.render({ page: 1, scale: 1.5, rotation: 90, clip: CLIP, appearance: "dark" });
    const [url] = asked;
    expect(url?.protocol).toBe("lamina:");
    expect(url?.pathname).toBe("/tile/4/1");
    expect(Object.fromEntries(url?.searchParams ?? [])).toEqual({
      s: "1.5",
      r: "90",
      a: "dark",
      x: "512",
      y: "0",
      w: "512",
      h: "300",
      p: "v",
      t: "1",
      fmt: "webp",
    });
    expect([drawn.width, drawn.height]).toEqual([512, 308]);
  });

  it("asks for the whole page when the request names no tile", async () => {
    const asked = serveTiles(tile);
    const engine = client();
    engine.show(PLAIN);
    await engine.render({ page: 0, scale: 1 });
    expect(["x", "y", "w", "h"].map((key) => asked[0]?.searchParams.get(key))).toEqual(["0", "0", "612", "792"]);
  });

  it("marks a render ahead of the scroll as prefetch", async () => {
    const asked = serveTiles(tile);
    const engine = client();
    engine.show(PLAIN);
    await engine.render({ page: 0, scale: 1, clip: CLIP }, { priority: "prefetch" });
    expect(asked[0]?.searchParams.get("p")).toBe("p");
  });

  it("rejects an aborted render with AbortError and cancels its ticket in the engine", async () => {
    serveTiles(tile, 1000);
    const engine = client();
    engine.show(PLAIN);
    const abort = new AbortController();
    const rendering = engine.render({ page: 0, scale: 1, clip: CLIP }, { signal: abort.signal });
    abort.abort();
    await expect(rendering).rejects.toMatchObject({ name: "AbortError" });
    await Promise.resolve();
    expect(commands).toContainEqual({ command: "cancel_tiles", args: { tickets: [1] } });
  });

  it("sends the renders aborted together in one cancel_tiles", async () => {
    serveTiles(tile, 1000);
    const engine = client();
    engine.show(PLAIN);
    const abort = new AbortController();
    const renders = [0, 1].map((page) => engine.render({ page, scale: 1, clip: CLIP }, { signal: abort.signal }));
    abort.abort();
    await Promise.allSettled(renders);
    await Promise.resolve();
    expect(commands.filter((c) => c.command === "cancel_tiles")).toEqual([
      { command: "cancel_tiles", args: { tickets: [1, 2] } },
    ]);
  });

  it("drops a tile of a document replaced while it travelled, without decoding it", async () => {
    serveTiles(tile, 10);
    const engine = client();
    engine.show(PLAIN);
    const rendering = engine.render({ page: 0, scale: 1, clip: CLIP });
    engine.show({ ...PLAIN, doc: 9 });
    await expect(rendering).rejects.toMatchObject({ name: "AbortError" });
    expect(bitmaps).toEqual([]);
  });

  it("closes a tile whose render was aborted while it decoded, and never returns it", async () => {
    serveTiles(tile);
    const abort = new AbortController();
    // The abort lands after the fetch answered, while the bitmap is being made.
    vi.stubGlobal("createImageBitmap", () => {
      const bitmap = new FakeBitmap(512, 300);
      bitmaps.push(bitmap);
      abort.abort();
      return Promise.resolve(bitmap);
    });
    const engine = client();
    engine.show(PLAIN);
    const rendering = engine.render({ page: 0, scale: 1, clip: CLIP }, { signal: abort.signal });
    await expect(rendering).rejects.toMatchObject({ name: "AbortError" });
    expect(bitmaps.map((b) => b.closed)).toEqual([true]);
  });

  it("reads a failure for a document replaced meanwhile as stale, not as an error", async () => {
    serveTiles(() => Response.json({ kind: "unknown-document", message: "no such document" }, { status: 404 }), 10);
    const engine = client();
    engine.show(PLAIN);
    const rendering = engine.render({ page: 0, scale: 1, clip: CLIP });
    engine.show({ ...PLAIN, doc: 9 });
    await expect(rendering).rejects.toMatchObject({ name: "AbortError" });
  });

  it("reads a tile the engine dropped as cancelled", async () => {
    serveTiles(() => Response.json({ kind: "cancelled", message: "cancelled" }, { status: 409 }));
    const engine = client();
    engine.show(PLAIN);
    await expect(engine.render({ page: 0, scale: 1, clip: CLIP })).rejects.toMatchObject({ name: "AbortError" });
  });

  it("rejects a failed tile with the main process's kind", async () => {
    serveTiles(() => Response.json({ kind: "page-out-of-range", message: "no such page" }, { status: 404 }));
    const engine = client();
    engine.show(PLAIN);
    await expect(engine.render({ page: 1, scale: 1, clip: CLIP })).rejects.toMatchObject({ code: "page-out-of-range" });
  });

  it("refuses a page the document doesn't have before fetching", async () => {
    const asked = serveTiles(tile);
    const engine = client();
    engine.show(PLAIN);
    await expect(engine.render({ page: 7, scale: 1, clip: CLIP })).rejects.toMatchObject({ code: "disposed" });
    expect(asked).toEqual([]);
  });

  it("reads a raw tile's pixels at the size the main process says", async () => {
    serveTiles(
      () => new Response(new Uint8Array(2 * 1 * 4), { headers: { "x-tile-width": "2", "x-tile-height": "1" } }),
    );
    const made: number[][] = [];
    const shell = {
      ...tauriShell(),
      fromPixels: (pixels: Uint8ClampedArray, width: number, height: number) => {
        made.push([pixels.length, width, height]);
        return Promise.resolve(new FakeBitmap(width, height) as unknown as ImageBitmap);
      },
    };
    const engine = new DesktopEngineClient(shell, { transport: "raw" });
    engine.show(PLAIN);
    await engine.render({ page: 0, scale: 1, clip: { x: 0, y: 0, width: 2, height: 1 } });
    expect(made).toEqual([[8, 2, 1]]);
  });
});

describe("the engine's events", () => {
  async function told(payload: RestartedPayload | undefined): Promise<string[]> {
    const engine = client();
    engine.show(PLAIN);
    const heard: string[] = [];
    engine.on("restarted", ({ reopened }) => heard.push(`restarted:${reopened}`));
    engine.on("password-required", () => heard.push("password-required"));
    engine.on("restart-failed", () => heard.push("restart-failed"));
    // The listeners register as the client starts.
    await new Promise((resolve) => setTimeout(resolve, 0));
    await (payload === undefined ? emit(FAILED_EVENT, { window: "main" }) : emit(RESTARTED_EVENT, payload));
    return heard;
  }

  it("redraws a document that reopened", async () => {
    expect(await told({ window: "main", docs: [{ doc: 4, state: "open" }] })).toEqual(["restarted:true"]);
  });

  it("asks for the password of a document that came back locked", async () => {
    expect(await told({ window: "main", docs: [{ doc: 4, state: "locked" }] })).toEqual(["password-required"]);
  });

  it("reports a document that didn't come back", async () => {
    expect(await told({ window: "main", docs: [{ doc: 4, state: "closed" }] })).toEqual(["restart-failed"]);
  });

  it("reports an engine that stays down", async () => {
    expect(await told(undefined)).toEqual(["restart-failed"]);
  });
});

describe("files to take pages from", () => {
  const insert = [{ op: "insertPagesFrom", args: { source: "abc", pages: ["p1.0"], at: "end" } }];

  it("draws a page of the file as any document's page, though another document shows", async () => {
    const asked = serveTiles(tile);
    const engine = client();
    engine.show(PLAIN);
    await engine.renderSource(OTHER, { page: 1, scale: 1, clip: CLIP });
    expect(asked[0]?.pathname).toBe("/tile/9/1");
  });

  it("makes an edit that took pages from the file again in a restarted engine, after giving it the file", async () => {
    const engine = client();
    engine.show(PLAIN);
    await engine.sources.add(PLAIN.doc, OTHER.doc);
    await engine.applyEdit(insert as never, { label: "Insert" });
    // The listeners register as the client starts.
    await new Promise((resolve) => setTimeout(resolve, 0));
    commands = [];
    await emit(RESTARTED_EVENT, { window: "main", docs: [{ doc: 4, state: "open" }] });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(commands.map((c) => c.command)).toEqual(["add_source", "edit_document"]);
  });

  it("closes the file with the document that took pages from it", async () => {
    const engine = client();
    engine.show(PLAIN);
    await engine.sources.add(PLAIN.doc, OTHER.doc);
    commands = [];
    await engine.close();
    expect(commands.map((c) => [c.command, (c.args as { doc: number }).doc])).toEqual([
      ["close_document", 9],
      ["close_document", 4],
    ]);
  });

  it("closes a file it was not given to a document, when the dialog lets it go", async () => {
    const engine = client();
    engine.show(PLAIN);
    const picked = await engine.sources.pick();
    commands = [];
    await engine.sources.release(picked?.doc ?? 0);
    expect(commands.map((c) => c.command)).toEqual(["close_document"]);
  });
});

describe("a copy with a change a signature refused", () => {
  const ops = [{ op: "rotatePage", args: { page: "p1.0", rotate: 90 } }];
  const options = { mode: "fullRewrite" as const, invalidateSignatures: true };

  /** The viewer's session as far as a copy goes: it runs the engine's step. */
  function session(engine: DesktopEngineClient) {
    return { copyWith: (o: unknown, opts: unknown) => engine.copyWithEdit(o, opts as never) };
  }

  it("asks the main process to write it, and gives the name the person chose", async () => {
    const engine = client();
    engine.show(PLAIN);
    commands = [];
    const name = await engine.copyWithTo(session(engine), ops as never, options);
    const sent = commands.find((c) => c.command === "save_copy_with");
    expect([name, sent?.args]).toEqual([
      "copy.pdf",
      {
        doc: 4,
        log: JSON.stringify({ v: 1, ops }),
        options: { rewrite: true, invalidateSignatures: true },
      },
    ]);
  });

  it("gives nothing when the person cancels the save dialog", async () => {
    copyName = null;
    const engine = client();
    engine.show(PLAIN);
    expect(await engine.copyWithTo(session(engine), ops as never, options)).toBeUndefined();
  });
});

describe("saving", () => {
  const options = { mode: "auto" as const, metadataDate: "2026-11-15T10:00:00.000Z" };

  it("asks the main process for the plan, with the options as it reads them", async () => {
    const engine = client();
    engine.show(PLAIN);
    const plan = await engine.planSave({
      mode: "fullRewrite",
      invalidateSignatures: true,
      instanceId: "00112233-4455-6677-8899-aabbccddeeff",
    });
    expect([commands.at(-1)?.args, plan.mode, plan.violations]).toEqual([
      {
        doc: 4,
        options: {
          rewrite: true,
          invalidateSignatures: true,
          instanceId: [0, 17, 34, 51, 68, 85, 102, 119, 136, 153, 170, 187, 204, 221, 238, 255],
        },
      },
      "incremental",
      [],
    ]);
  });

  it("saves in place through the edit session, which then counts nothing unsaved", async () => {
    saveAnswer = savedAs("letter.pdf");
    const engine = client();
    engine.show(PLAIN);
    await engine.applyEdit([] as never, { label: "Rotate page 1" });
    const saved = await engine.saveTo(session(engine), "in-place", options);
    const history = await engine.editHistory();
    expect([commands.at(-1)?.command, saved?.name, history.unsaved, engine.unsavedDocs]).toEqual([
      "save_document",
      "letter.pdf",
      0,
      [],
    ]);
  });

  it("saves the document it was asked for, even if another tab shows by the time the save runs", async () => {
    saveAnswer = savedAs("letter.pdf");
    const engine = client();
    engine.show(PLAIN);
    let go: () => void = () => undefined;
    const waited = new Promise<void>((resolve) => {
      go = resolve;
    });
    const later = {
      async save(options: Parameters<DesktopEngineClient["copyEdits"]>[0], write: () => Promise<void>) {
        await waited;
        return session(engine).save(options, write);
      },
    };
    const saving = engine.saveTo(later, "in-place", options);
    engine.show({ ...PLAIN, doc: 9, name: "other.pdf" });
    go();
    await saving;
    expect(commands.find((c) => c.command === "save_document")?.args).toMatchObject({ doc: 4 });
  });

  it("names the document after the file Save as wrote", async () => {
    saveAnswer = savedAs("letter copy.pdf");
    const engine = client();
    engine.show(PLAIN);
    await engine.applyEdit([] as never, { label: "Rotate page 1" });
    await engine.saveTo(session(engine), "new-file", options);
    expect([commands.at(-1)?.command, engine.current?.name]).toEqual(["save_document_as", "letter copy.pdf"]);
  });

  it("saves nothing when Save as's dialog is cancelled", async () => {
    saveAnswer = null;
    const engine = client();
    engine.show(PLAIN);
    await engine.applyEdit([] as never, { label: "Rotate page 1" });
    const saved = await engine.saveTo(session(engine), "new-file", options);
    expect([saved, (await engine.editHistory()).unsaved]).toEqual([undefined, 1]);
  });

  it("keeps the edit unsaved when the write fails, with the main process's reason", async () => {
    saveAnswer = new Error("refused", { cause: { kind: "write-refused", message: "the file could not be written" } });
    const engine = client();
    engine.show(PLAIN);
    await engine.applyEdit([] as never, { label: "Rotate page 1" });
    await expect(engine.saveTo(session(engine), "overwrite", options)).rejects.toMatchObject({ code: "write-refused" });
    expect([commands.at(-1)?.args, (await engine.editHistory()).unsaved]).toEqual([
      {
        doc: 4,
        options: { rewrite: false, invalidateSignatures: false, metadataDate: options.metadataDate },
        overwrite: true,
      },
      1,
    ]);
  });
});

describe("the opener", () => {
  const viewer = () => ({ open: vi.fn() });

  it("shows a plain document at once", async () => {
    const view = viewer();
    const engine = client();
    const opened = await desktopOpener(tauriShell(), engine, view as never)(desktopFile(PLAIN), undefined);
    expect([opened.pageCount, view.open.mock.calls.length, engine.current?.doc]).toEqual([2, 1, 4]);
  });

  it("asks for a locked document's password", async () => {
    const open = desktopOpener(tauriShell(), client(), viewer() as never);
    await expect(open(desktopFile(LOCKED), undefined)).rejects.toMatchObject({ code: "password-required" });
  });

  it("reports a refused password as wrong", async () => {
    const open = desktopOpener(tauriShell(), client(), viewer() as never);
    await expect(open(desktopFile(LOCKED), "nope")).rejects.toMatchObject({ code: "wrong-password" });
  });

  it("unlocks with the right password and shows the unlocked document", async () => {
    const engine = client();
    const opened = await desktopOpener(tauriShell(), engine, viewer() as never)(desktopFile(LOCKED), "secret");
    expect([opened.pageCount, engine.current?.name, engine.current?.needsPassword]).toEqual([2, "letter.pdf", false]);
    expect(commands).toContainEqual({ command: "unlock_document", args: { doc: 5, password: "secret" } });
  });
});

describe("picking a document", () => {
  it("hands the reader a file that fails to open with the main process's reason", async () => {
    mockIPC((command) => {
      if (command === "open_dialog")
        return Promise.reject({ kind: "invalid", message: "this file isn't a PDF Lamina PDF can read" });
      return undefined;
    });
    const { pickDocument } = await import("./open.ts");
    const [file] = await pickDocument(tauriShell());
    const open = desktopOpener(tauriShell(), client(), { open: vi.fn() } as never);
    await expect(open(file as never, undefined)).rejects.toMatchObject({ code: "invalid" });
  });

  it("hands nothing back when the user cancels", async () => {
    mockIPC(() => null);
    const { pickDocument } = await import("./open.ts");
    await expect(pickDocument(tauriShell())).resolves.toEqual([]);
  });
});

describe("the wire, as the main process reads it", () => {
  // src/engine/wire.fixture.json: the Rust side's tests read the same file.
  const wire = wireFixture as {
    events: { restarted: string; failed: string };
    reopened: string[];
    cancelled: { kind: string; status: number };
    tiles: { address: TileAddress; path: string; query: string }[];
  };

  it("builds each tile address the main process parses", () => {
    for (const { address, path, query } of wire.tiles)
      expect(tileUrl("lamina://localhost/", address)).toBe(`lamina://localhost${path}?${query}`);
  });

  it("listens to the main process's event names", () => {
    expect([RESTARTED_EVENT, FAILED_EVENT]).toEqual([wire.events.restarted, wire.events.failed]);
  });

  it("reads the main process's cancelled tile as an abort", async () => {
    const { kind, status } = wire.cancelled;
    serveTiles(() => Response.json({ kind, message: "cancelled" }, { status }));
    const engine = client();
    engine.show(PLAIN);
    await expect(engine.render({ page: 0, scale: 1, clip: CLIP })).rejects.toMatchObject({ name: "AbortError" });
  });

  it("knows every state a document comes back in after a restart", async () => {
    const heard: string[] = [];
    const engine = client();
    engine.show(PLAIN);
    engine.on("restarted", () => heard.push("restarted"));
    engine.on("password-required", () => heard.push("password-required"));
    engine.on("restart-failed", () => heard.push("restart-failed"));
    await new Promise((resolve) => setTimeout(resolve, 0));
    for (const state of wire.reopened) await emit(RESTARTED_EVENT, { window: "main", docs: [{ doc: 4, state }] });
    expect(heard).toEqual(["restarted", "password-required", "restart-failed"]);
  });
});
