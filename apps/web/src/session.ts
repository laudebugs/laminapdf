// The reader's engine: one document worker and the view over it (a session), the controller that
// runs the reader's lifecycle, and what the chrome shows about the open document. The backend's
// WebAssembly starts loading as soon as the app does, so a file chosen meanwhile waits only for
// what is left. Switching backend (the #debug menu) starts a new session and reopens the file.

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import type { DebugBackend, ExternalStore, OpenedDocument, Platform, PlatformFile, RepairKind } from "@laminapdf/ui";
import { ReaderController } from "@laminapdf/ui";
import { createViewer, type Viewer, ViewerClient } from "@laminapdf/viewer";
import { type DocumentWarning, repairsFrom } from "./document.ts";

/** One backend's document worker and the view over it. */
interface Session {
  readonly backend: DebugBackend;
  readonly client: ViewerClient;
  readonly viewer: Viewer;
}

/** What the chrome shows besides the reader's lifecycle. */
export interface EngineState {
  readonly backend: DebugBackend;
  /** The view, once its backend has loaded. */
  readonly viewer: Viewer | undefined;
  /** The open document's metadata title, when it has one. */
  readonly title: string | undefined;
  /** The open document's repairs, one per warning code. */
  readonly repairs: readonly RepairKind[];
  /** For #debug: the backend's name and version, and how long the open took to paint page 1. */
  readonly timing: { readonly backend: string; readonly firstPageMs: number } | undefined;
}

/** Starts `backend`'s document worker and makes a view over it. */
async function startSession(backend: DebugBackend): Promise<Session> {
  const client = await ViewerClient.create({
    backend: backend === "pdfium" ? { backend: "pdfium", wasm: pdfiumWasm } : { backend: "hayro" },
  });
  return { backend, client, viewer: createViewer({ client }) };
}

/** What the document worker opens for `file`: the File itself, never a copy of its bytes. */
async function sourceOf(file: PlatformFile): Promise<Blob | Uint8Array> {
  const { handle } = file;
  if (handle instanceof Blob) return handle;
  if (handle !== undefined) return handle.getFile();
  return file.read(0, file.size);
}

/** Resolves true on the first animation frame where `done()` holds, or false once `stale()` does. */
function whenPainted(done: () => boolean, stale: () => boolean): Promise<boolean> {
  return new Promise((resolve) => {
    const check = () => {
      if (stale()) resolve(false);
      else if (done()) resolve(true);
      else requestAnimationFrame(check);
    };
    requestAnimationFrame(check);
  });
}

/** The reader's engine, as an external store for React (`useStoreState`). */
export class Engine implements ExternalStore<EngineState> {
  /** The reader's lifecycle: the chrome's controls call it, and it calls `open` below. */
  readonly reader: ReaderController;
  readonly #listeners = new Set<() => void>();
  #session: Promise<Session>;
  /** The file open now, for reopening it in another backend. */
  #file: PlatformFile | undefined;
  /** Counts opens and backend switches: an open that isn't the latest shows and closes nothing. */
  #turn = 0;
  /**
   * Warnings received since the last open answered. The worker opens one file at a time and sends
   * a file's warnings before its answer, so each answer takes exactly its own.
   */
  #incoming: DocumentWarning[] = [];
  #state: EngineState;

  constructor(platform: Platform, backend: DebugBackend) {
    this.reader = new ReaderController(platform, (file, password) => this.#open(file, password));
    this.#state = { backend, viewer: undefined, title: undefined, repairs: [], timing: undefined };
    this.#session = this.#start(backend);
  }

  getState = (): EngineState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Moves to `backend`: a new document worker, and the open file reopened in it. */
  async switchBackend(backend: DebugBackend): Promise<void> {
    if (backend === this.#state.backend) return;
    const old = this.#session;
    const file = this.#file;
    this.reader.close();
    this.#turn += 1;
    this.#set({ backend, viewer: undefined });
    this.#session = this.#start(backend);
    void old.then(({ client, viewer }) => {
      viewer.destroy();
      client.terminate();
    });
    if (file !== undefined) await this.reader.open(file);
  }

  #start(backend: DebugBackend): Promise<Session> {
    const session = startSession(backend);
    session.then(
      (started) => {
        if (this.#session !== session) return;
        started.client.on("warning", (warning) => {
          this.#incoming.push(warning as DocumentWarning);
        });
        this.#set({ viewer: started.viewer });
      },
      // A backend that can't load fails the next open, which the reader reports.
      () => undefined,
    );
    return session;
  }

  async #open(file: PlatformFile, password: string | undefined): Promise<OpenedDocument> {
    this.#turn += 1;
    const turn = this.#turn;
    const latest = () => turn === this.#turn;
    const session = await this.#session;
    const started = performance.now();
    let opened: Awaited<ReturnType<ViewerClient["open"]>>;
    let warnings: DocumentWarning[];
    try {
      opened = await session.client.open(await sourceOf(file), password === undefined ? {} : { password });
    } finally {
      warnings = this.#incoming.splice(0);
    }
    // A later open or a backend switch came first: its own open replaces this one in the worker,
    // so nothing here is shown, and closing it must not close the document the worker holds now.
    if (!latest()) return { pageCount: opened.pageCount };
    session.viewer.open(opened);
    this.#file = file;
    this.#set({ title: opened.title, repairs: repairsFrom(warnings), timing: undefined });
    const firstPage = () => {
      const state = session.viewer.stats().pages[0];
      return state === "low" || state === "sharp" || state === "error";
    };
    void whenPainted(firstPage, () => !latest()).then((painted) => {
      if (!painted) return;
      const backend = `${opened.backend.name} ${opened.backend.version}`;
      this.#set({ timing: { backend, firstPageMs: Math.round(performance.now() - started) } });
    });
    return {
      pageCount: opened.pageCount,
      close: () => {
        if (!latest()) return;
        this.#file = undefined;
        this.#set({ title: undefined, repairs: [], timing: undefined });
        if (session.client.document?.id === opened.id) void session.client.close().catch(() => undefined);
      },
    };
  }

  #set(next: Partial<EngineState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}
