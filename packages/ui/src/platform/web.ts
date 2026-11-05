// The web Platform (ADR 0010): each capability feature-detected once, from the scope it is given
// (the window by default, a fake in tests), never from the user agent. Pickers where the browser
// has them, an <input type=file> and downloads where it doesn't; storage in IndexedDB and OPFS.

import { message } from "../messages.js";
import { createIdbStore, type KeyValueStore } from "./idb.js";
import type {
  Capabilities,
  Capability,
  Platform,
  PlatformFile,
  PlatformStorage,
  RecentFile,
  RecentFiles,
  SaveResult,
} from "./types.js";

/** The parts of a window the web platform uses: tests pass a fake with only what they need. */
export interface WebScope {
  readonly document: Document;
  readonly navigator: Navigator;
  readonly indexedDB?: IDBFactory;
  readonly crossOriginIsolated?: boolean;
  readonly matchMedia?: (query: string) => { readonly matches: boolean };
  readonly showOpenFilePicker?: (options?: object) => Promise<FileSystemFileHandle[]>;
  readonly showSaveFilePicker?: (options?: object) => Promise<FileSystemFileHandle>;
  readonly launchQueue?: {
    setConsumer(consumer: (params: { readonly files: readonly FileSystemHandle[] }) => void): void;
  };
  readonly FileSystemFileHandle?: { readonly prototype: object };
  readonly FileSystemHandle?: { readonly prototype: object };
  readonly URL: { createObjectURL(blob: Blob): string; revokeObjectURL(url: string): void };
  readonly open: (url: string, target: string, features: string) => unknown;
}

/** How many recent documents are kept. */
const RECENT_LIMIT = 20;
const PDF_TYPES = [{ description: message("platform.picker.pdf"), accept: { "application/pdf": [".pdf"] } }];

/** Detects the web values of every capability (ADR 0010 §2), once. */
export function detectCapabilities(scope: WebScope): Capabilities {
  const pickers = typeof scope.showOpenFilePicker === "function";
  const storage = scope.navigator.storage as Partial<StorageManager> | undefined;
  const installed = ["(display-mode: standalone)", "(display-mode: window-controls-overlay)"].some(
    (query) => scope.matchMedia?.(query).matches === true,
  );
  return {
    canWriteInPlace: pickers && hasMethod(scope.FileSystemFileHandle?.prototype, "createWritable"),
    canSaveAsDialog: typeof scope.showSaveFilePicker === "function",
    canWatchFiles: false,
    hasNativePrint: false,
    multiWindow: false,
    hasFileHandlers: scope.launchQueue !== undefined && installed,
    canReopenRecent:
      pickers && scope.indexedDB !== undefined && hasMethod(scope.FileSystemHandle?.prototype, "requestPermission"),
    hasOpfs: typeof storage?.getDirectory === "function",
    canPersistStorage: typeof storage?.persist === "function",
    hasNativeMenus: false,
    canOpenUrl: true,
    crossOriginIsolated: scope.crossOriginIsolated === true,
    ownsKeyboard: false,
  };
}

/** The web Platform over `scope` (the window by default). */
export function createWebPlatform(scope: WebScope = globalThis as unknown as WebScope): Platform {
  const capabilities = detectCapabilities(scope);
  const store = scope.indexedDB === undefined ? memoryStore() : createIdbStore(scope.indexedDB);
  const listeners = new Set<(files: PlatformFile[]) => void>();
  if (capabilities.hasFileHandlers) {
    scope.launchQueue?.setConsumer(async ({ files }) => {
      const opened = await Promise.all(files.filter(isFileHandle).map((handle) => fromHandle(handle, capabilities)));
      for (const listener of listeners) listener(opened);
    });
  }
  return {
    capabilities,
    howToEnable: (flag) => howToEnable(flag, capabilities, scope),
    async openFiles({ multiple }) {
      if (scope.showOpenFilePicker === undefined)
        return (await pickWithInput(scope.document, multiple)).map((f) => fromFile(f));
      try {
        const handles = await scope.showOpenFilePicker({ multiple, types: PDF_TYPES, excludeAcceptAllOption: false });
        return await Promise.all(handles.map((handle) => fromHandle(handle, capabilities)));
      } catch (error) {
        if (isAbort(error)) return [];
        throw error;
      }
    },
    filesFromDataTransfer: (data) => filesFromDataTransfer(data, capabilities),
    onOpenRequest(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    saveAs: (bytes, name) => saveAs(scope, bytes, name),
    async saveInPlace(file, bytes) {
      const handle = file.handle;
      if (!capabilities.canWriteInPlace || !file.writable || !isFileHandle(handle)) {
        throw new Error(message("platform.error.notWritable"));
      }
      await write(handle, bytes);
      return { saved: true, name: file.name };
    },
    print: () => Promise.reject(new Error(message("platform.error.print"))),
    copyText: (text) => scope.navigator.clipboard.writeText(text),
    async openExternal(url) {
      const { protocol } = new URL(url);
      if (!["http:", "https:", "mailto:"].includes(protocol)) throw new Error(message("platform.error.externalScheme"));
      scope.open(url, "_blank", "noopener,noreferrer");
    },
    storage: webStorage(scope, store),
    recent: recentFiles(store, capabilities),
  };
}

function howToEnable(flag: Capability, capabilities: Capabilities, scope: WebScope): string | undefined {
  if (capabilities[flag]) return undefined;
  switch (flag) {
    case "canWriteInPlace":
    case "canSaveAsDialog":
      return message("platform.enable.savePicker");
    case "canReopenRecent":
      return message("platform.enable.reopen");
    case "hasFileHandlers":
      // The browser could open files once the app is installed; another browser never would.
      return scope.launchQueue === undefined ? undefined : message("platform.enable.install");
    default:
      return undefined;
  }
}

/** A PlatformFile over a File: range reads by slicing, the File itself for the worker. */
export function fromFile(file: File, handle?: FileSystemFileHandle, writable = false): PlatformFile {
  return {
    name: file.name,
    size: file.size,
    read: async (offset, length) => new Uint8Array(await file.slice(offset, offset + length).arrayBuffer()),
    handle: handle ?? file,
    writable,
  };
}

async function fromHandle(handle: FileSystemFileHandle, capabilities: Capabilities): Promise<PlatformFile> {
  return fromFile(await handle.getFile(), handle, capabilities.canWriteInPlace);
}

/**
 * Files from a drop or a paste. Handles are asked for before anything is awaited, since a
 * DataTransfer's items empty once the event's task ends; without handles, the plain files.
 */
function filesFromDataTransfer(data: DataTransfer, capabilities: Capabilities): Promise<PlatformFile[]> {
  const items = [...data.items].filter((item) => item.kind === "file");
  const asHandles = capabilities.canWriteInPlace && items.every((item) => "getAsFileSystemHandle" in item);
  if (!asHandles) return Promise.resolve([...data.files].map((file) => fromFile(file)));
  const pending = items.map((item) =>
    (item as DataTransferItem & { getAsFileSystemHandle(): Promise<FileSystemHandle | null> }).getAsFileSystemHandle(),
  );
  return Promise.all(pending).then((handles) =>
    Promise.all(handles.filter(isFileHandle).map((handle) => fromHandle(handle, capabilities))),
  );
}

/** A hidden file input's choice; [] when the user cancels. */
function pickWithInput(doc: Document, multiple: boolean): Promise<File[]> {
  return new Promise((resolve) => {
    const input = doc.createElement("input");
    input.type = "file";
    input.accept = ".pdf,application/pdf";
    input.multiple = multiple;
    input.addEventListener("change", () => resolve([...(input.files ?? [])]), { once: true });
    input.addEventListener("cancel", () => resolve([]), { once: true });
    input.click();
  });
}

async function saveAs(scope: WebScope, bytes: Uint8Array, suggestedName: string): Promise<SaveResult> {
  if (scope.showSaveFilePicker === undefined) {
    // No dialog: the browser downloads it, under the suggested name.
    const url = scope.URL.createObjectURL(new Blob([bytes as BlobPart], { type: "application/pdf" }));
    const link = scope.document.createElement("a");
    link.href = url;
    link.download = suggestedName;
    link.click();
    setTimeout(() => scope.URL.revokeObjectURL(url), 0);
    return { saved: true, name: suggestedName };
  }
  try {
    const handle = await scope.showSaveFilePicker({ suggestedName, types: PDF_TYPES });
    await write(handle, bytes);
    return { saved: true, name: handle.name };
  } catch (error) {
    if (isAbort(error)) return { saved: false, reason: "cancelled" };
    throw error;
  }
}

async function write(handle: FileSystemFileHandle, bytes: Uint8Array): Promise<void> {
  const writable = await handle.createWritable();
  await writable.write(bytes as FileSystemWriteChunkType);
  await writable.close();
}

function webStorage(scope: WebScope, store: KeyValueStore): PlatformStorage {
  const storage = scope.navigator.storage as Partial<StorageManager> | undefined;
  return {
    opfs: async () => (typeof storage?.getDirectory === "function" ? storage.getDirectory() : undefined),
    get: (key) => store.get(key),
    set: (key, value) => store.set(key, value),
    delete: (key) => store.delete(key),
    persist: async () => (typeof storage?.persist === "function" ? storage.persist() : false),
    clear: () => store.clear(),
  };
}

const RECENT = "recent";
const handleKey = (id: string) => `recent-handle:${id}`;

/** Recent documents in the key-value store; their file handles beside them where they can reopen. */
function recentFiles(store: KeyValueStore, capabilities: Capabilities): RecentFiles {
  const list = async () => ((await store.get(RECENT)) as RecentFile[] | undefined) ?? [];
  const recent: RecentFiles = {
    list,
    async add(file) {
      const entries = await list();
      const same = entries.find((entry) => entry.name === file.name && entry.size === file.size);
      const id = same?.id ?? crypto.randomUUID();
      const entry: RecentFile = { id, name: file.name, size: file.size, lastOpened: Date.now() };
      const kept = [entry, ...entries.filter((e) => e.id !== id)];
      for (const dropped of kept.splice(RECENT_LIMIT)) await store.delete(handleKey(dropped.id));
      await store.set(RECENT, kept);
      if (capabilities.canReopenRecent && isFileHandle(file.handle)) await store.set(handleKey(id), file.handle);
    },
    async remove(id) {
      await store.set(
        RECENT,
        (await list()).filter((entry) => entry.id !== id),
      );
      await store.delete(handleKey(id));
    },
    async clear() {
      for (const entry of await list()) await store.delete(handleKey(entry.id));
      await store.delete(RECENT);
    },
  };
  if (capabilities.canReopenRecent) {
    recent.reopen = async (id) => {
      const handle = await store.get(handleKey(id));
      if (!isFileHandle(handle) || !(await canRead(handle))) return undefined;
      return fromHandle(handle, capabilities);
    };
  }
  return recent;
}

/**
 * Whether `handle` may be read: asked first, so a browser that kept the permission shows no
 * prompt; requested otherwise, which needs the click that called `reopen`.
 */
async function canRead(handle: FileSystemFileHandle): Promise<boolean> {
  const permitted = handle as FileSystemFileHandle & {
    queryPermission?(options: object): Promise<PermissionState>;
    requestPermission(options: object): Promise<PermissionState>;
  };
  const read = { mode: "read" };
  if ((await permitted.queryPermission?.(read)) === "granted") return true;
  return (await permitted.requestPermission(read)) === "granted";
}

/** A store in memory, for a browser without IndexedDB (private modes of some). */
function memoryStore(): KeyValueStore {
  const map = new Map<string, unknown>();
  return {
    get: async (key) => map.get(key),
    set: async (key, value) => void map.set(key, value),
    delete: async (key) => void map.delete(key),
    clear: async () => map.clear(),
  };
}

function isFileHandle(value: unknown): value is FileSystemFileHandle {
  return (
    typeof value === "object" && value !== null && (value as { kind?: unknown }).kind === "file" && "getFile" in value
  );
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function hasMethod(prototype: object | undefined, name: string): boolean {
  return prototype !== undefined && typeof (prototype as Record<string, unknown>)[name] === "function";
}
