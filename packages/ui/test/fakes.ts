// Fakes the tests share: a Platform with any combination of capability flags (ADR 0010: every
// combination testable without a browser matrix), files from bytes, and a stand-in window for
// the web platform's feature detection.

import type { Capabilities, Platform, PlatformFile, RecentFile } from "../src/platform/types.ts";
import type { WebScope } from "../src/platform/web.ts";
import type { FindOptionsSlice, FindSlice } from "../src/reader/find-bar.tsx";
import type { FindResultItem } from "../src/reader/find-results.tsx";
import type { ViewerSlice, Zoom } from "../src/reader/viewer-controls.ts";

/** Every flag false: a platform that can do nothing optional. */
const NO_CAPABILITIES: Capabilities = {
  canWriteInPlace: false,
  canSaveAsDialog: false,
  canWatchFiles: false,
  hasNativePrint: false,
  multiWindow: false,
  hasFileHandlers: false,
  canReopenRecent: false,
  hasOpfs: false,
  canPersistStorage: false,
  hasNativeMenus: false,
  canOpenUrl: false,
  crossOriginIsolated: false,
  ownsKeyboard: false,
  canShare: false,
};

/** A file over `bytes`, as a picker or a drop would give it. */
export function fakeFile(name = "report.pdf", bytes = new Uint8Array([37, 80, 68, 70])): PlatformFile {
  return {
    name,
    size: bytes.length,
    read: async (offset, length) => bytes.slice(offset, offset + length),
    writable: false,
  };
}

/** What a fake platform was asked to do, for assertions. */
export interface FakeCalls {
  readonly openFiles: { multiple: boolean }[];
  readonly copied: string[];
  /** What `shareText` was given. */
  readonly shared: string[];
}

/**
 * A Platform whose flags are `flags` over NO_CAPABILITIES, whose picker answers with `picked`,
 * and whose `howToEnable` answers from `reasons` (undefined: this platform never can).
 */
export function fakePlatform(
  options: {
    flags?: Partial<Capabilities>;
    reasons?: Partial<Record<keyof Capabilities, string>>;
    picked?: PlatformFile[];
  } = {},
): Platform & { readonly calls: FakeCalls } {
  const calls: FakeCalls = { openFiles: [], copied: [], shared: [] };
  const capabilities = { ...NO_CAPABILITIES, ...options.flags };
  const values = new Map<string, unknown>();
  let recent: RecentFile[] = [];
  return {
    calls,
    capabilities,
    howToEnable: (flag) => (capabilities[flag] ? undefined : options.reasons?.[flag]),
    async openFiles(request) {
      calls.openFiles.push({ ...request });
      return options.picked ?? [];
    },
    filesFromDataTransfer: async (data) =>
      [...data.files].map((file) => ({
        name: file.name,
        size: file.size,
        read: async (offset, length) => new Uint8Array(await file.slice(offset, offset + length).arrayBuffer()),
        handle: file,
        writable: false,
      })),
    onOpenRequest: () => () => undefined,
    saveAs: async (_bytes, name) => ({ saved: true, name }),
    saveInPlace: async (file) => ({ saved: true, name: file.name }),
    print: async () => undefined,
    async copyText(text) {
      calls.copied.push(text);
    },
    async shareText(text) {
      calls.shared.push(text);
    },
    openExternal: async () => undefined,
    storage: {
      opfs: async () => undefined,
      get: async (key) => values.get(key),
      set: async (key, value) => void values.set(key, value),
      delete: async (key) => void values.delete(key),
      persist: async () => false,
      clear: async () => values.clear(),
    },
    recent: {
      list: async () => recent,
      async add(file) {
        recent = [{ id: file.name, name: file.name, size: file.size, lastOpened: 0 }, ...recent];
      },
      async remove(id) {
        recent = recent.filter((entry) => entry.id !== id);
      },
      async clear() {
        recent = [];
      },
    },
  };
}

/** A window with only what `parts` gives, plus a document, navigator, URL and open that record. */
export function fakeScope(parts: Partial<WebScope> = {}): WebScope & { readonly opened: string[] } {
  const opened: string[] = [];
  return {
    document: {} as Document,
    navigator: {} as Navigator,
    URL: { createObjectURL: () => "blob:fake", revokeObjectURL: () => undefined },
    open: (url: string) => void opened.push(url),
    opened,
    ...parts,
  };
}

/**
 * An IndexedDB with one database and one store, in memory: enough for the web platform's
 * key-value store. Values are kept as given, not cloned, so fake file handles survive. With
 * `failOpen` the database won't open, as in some private modes; with `blockOpen` another tab
 * holds it at an older version; with `abortWrites` every write's
 * request succeeds and its transaction then aborts, as when the origin is out of quota.
 */
export function fakeIndexedDb(
  options: { failOpen?: boolean; blockOpen?: boolean; abortWrites?: boolean } = {},
): IDBFactory {
  const data = new Map<unknown, unknown>();
  type Handlers = Record<string, unknown> & { onsuccess?: () => void; onerror?: () => void };
  type Transaction = { oncomplete?: () => void; onabort?: () => void; error?: DOMException };
  const transaction = (mode: string) => {
    const tx: Transaction & { objectStore: () => unknown } = { objectStore: () => store };
    const request = (act: () => unknown) => {
      const pending: Handlers = {};
      queueMicrotask(() => {
        pending.result = act();
        pending.onsuccess?.();
        if (mode === "readwrite" && options.abortWrites === true) {
          tx.error = new DOMException("Quota exceeded", "QuotaExceededError");
          tx.onabort?.();
        } else tx.oncomplete?.();
      });
      return pending;
    };
    const store = {
      get: (key: unknown) => request(() => data.get(key)),
      put: (value: unknown, key: unknown) => request(() => void data.set(key, value)),
      delete: (key: unknown) => request(() => void data.delete(key)),
      clear: () => request(() => data.clear()),
    };
    return tx;
  };
  const database = { transaction: (_name: string, mode: string) => transaction(mode), createObjectStore: () => ({}) };
  return {
    open: () => {
      const opening: Handlers & { onupgradeneeded?: () => void; onblocked?: () => void } = {};
      queueMicrotask(() => {
        if (options.failOpen === true) return opening.onerror?.();
        if (options.blockOpen === true) return opening.onblocked?.();
        opening.result = database;
        opening.onupgradeneeded?.();
        opening.onsuccess?.();
      });
      return opening;
    },
  } as unknown as IDBFactory;
}

/** A viewer the chrome can drive: its state as a store, and every call it was asked to make. */
export class FakeViewer {
  readonly calls: { method: string; value: unknown }[] = [];
  #state: ViewerSlice;
  readonly #listeners = new Set<() => void>();

  constructor(state: Partial<ViewerSlice> = {}) {
    this.#state = { pageCount: 24, currentPage: 2, zoom: 110, percent: 110, ...state };
  }

  getState(): ViewerSlice {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Changes the state as the viewer would after a scroll or a zoom, and tells subscribers. */
  set(next: Partial<ViewerSlice>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }

  goToPage(index: number): void {
    this.calls.push({ method: "goToPage", value: index });
  }

  setZoom(zoom: Zoom): void {
    this.calls.push({ method: "setZoom", value: zoom });
  }

  rotate(delta: number): void {
    this.calls.push({ method: "rotate", value: delta });
  }
}

/** Find as the find bar drives it: a state that tests set, and the calls it was asked to make. */
export class FakeFind {
  readonly calls: { query: string; options: FindOptionsSlice | undefined }[] = [];
  #state: FindSlice;
  readonly #listeners = new Set<() => void>();

  constructor(state: Partial<FindSlice> = {}) {
    this.#state = {
      query: "",
      options: {},
      total: 0,
      current: undefined,
      searched: 0,
      pageCount: 24,
      done: true,
      pagesWithoutText: 0,
      ...state,
    };
  }

  getState(): FindSlice {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Changes the state as results arrive, and tells subscribers. */
  set(next: Partial<FindSlice>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }

  find(query: string, options?: FindOptionsSlice): void {
    this.calls.push({ query, options });
  }

  /** The matches the Results tab lists; tests set them. */
  list: readonly FindResultItem[] = [];
  readonly went: number[] = [];

  results(): readonly FindResultItem[] {
    return this.list;
  }

  goTo(ordinal: number): void {
    this.went.push(ordinal);
    this.set({ current: ordinal });
  }
}
