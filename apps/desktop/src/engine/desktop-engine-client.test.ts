// The desktop engine client over Tauri's mocked IPC and events (@tauri-apps/api/mocks): the real
// `tauriShell()`, with only the webview's `fetch` and `createImageBitmap` stood in for, since Node
// has neither the `lamina://` scheme nor a decoder.

import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockConvertFileSrc, mockIPC } from "@tauri-apps/api/mocks";
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

const LETTER = { width: 612, height: 792, rotate: 0 };

const PLAIN: DesktopDocument = {
  doc: 4,
  name: "letter.pdf",
  size: 1234,
  pageCount: 2,
  encrypted: false,
  needsPassword: false,
  repaired: false,
  pages: [LETTER, LETTER],
};

const LOCKED: DesktopDocument = { ...PLAIN, doc: 5, encrypted: true, needsPassword: true, pageCount: 0, pages: [] };

/** A bitmap that records whether it was closed. */
class FakeBitmap {
  closed = false;
  constructor(
    readonly width: number,
    readonly height: number,
  ) {}
  close(): void {
    this.closed = true;
  }
}

let commands: { command: string; args: unknown }[] = [];
let bitmaps: FakeBitmap[] = [];

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
  commands = [];
  bitmaps = [];
  mockConvertFileSrc("macos");
  mockIPC(
    (command, args) => {
      commands.push({ command, args });
      if (command === "unlock_document") {
        const { password } = args as { password: string };
        if (password !== "secret")
          return Promise.reject({ kind: "wrong-password", message: "the password doesn't open this document" });
        return { ...LOCKED, needsPassword: false, pageCount: 2, pages: [LETTER, LETTER], name: "", size: 0 };
      }
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

  it("closes a tile of a document replaced while it travelled, and never returns it", async () => {
    serveTiles(tile, 10);
    const engine = client();
    engine.show(PLAIN);
    const rendering = engine.render({ page: 0, scale: 1, clip: CLIP });
    engine.show({ ...PLAIN, doc: 9 });
    await expect(rendering).rejects.toMatchObject({ name: "AbortError" });
    expect(bitmaps.map((b) => b.closed)).toEqual([true]);
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

  it("unlocks with the right password and keeps the file's name", async () => {
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
