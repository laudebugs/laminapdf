// A large local file read from the origin-private file system: copied there once, in the document
// worker, then read through a synchronous access handle, so neither Lamina PDF nor the render
// backend (PDFium reads synchronously) needs the whole file in memory. Copies live in one folder,
// named by the file's size, date and name, so opening the same file again reads the copy already
// there; when the folder would pass its budget, the copies used longest ago go first. OPFS is a
// cache, never the only copy: a browser without it, a copy that doesn't fit or fails partway, or a
// file another tab holds opens from memory, as before; only a cancel stops the open.

import type { RenderSource } from "@laminapdf/render";
import type { RangeSource } from "@laminapdf/sdk";

/** The OPFS folder of document copies, which the app's "Clear recent files and cached documents" empties too. */
export const CACHE_FOLDER = "documents";

/** Files above this many bytes are read from a copy: 200 MiB. */
export const COPY_ABOVE = 200 * 1024 * 1024;

/** How much the copies may take, at most: 2 GiB, and never more than half the free quota. */
const MAX_CACHE_BYTES = 2 * 1024 * 1024 * 1024;

/** Copied in steps of 8 MiB, each a chance to stop and to report progress. */
const CHUNK = 8 * 1024 * 1024;

/** Beside each copy, a file holding when it was last opened (the copy's own date is when it was made). */
const USED = ".used";

/** The parts of a sync access handle used here (dedicated workers only). */
interface SyncHandle {
  read(buffer: Uint8Array, options: { at: number }): number;
  write(buffer: Uint8Array, options: { at: number }): number;
  truncate(size: number): void;
  getSize(): number;
  flush(): void;
  close(): void;
}

type SyncFileHandle = FileSystemFileHandle & { createSyncAccessHandle(): Promise<SyncHandle> };

/** The copy's name: its size, date and a hash of its name, so the same file finds its copy again. */
export function copyName(file: Pick<File, "name" | "size" | "lastModified">): string {
  let hash = 2166136261;
  for (let i = 0; i < file.name.length; i++) hash = Math.imul(hash ^ file.name.charCodeAt(i), 16777619);
  return `${file.size}-${file.lastModified}-${(hash >>> 0).toString(16)}.pdf`;
}

/** How a copy is made and kept. */
export interface CopyOptions {
  readonly signal: AbortSignal;
  /** Told after each step of a copy being made. */
  readonly onProgress?: ((loaded: number, total: number) => void) | undefined;
  /** Keep the copy for next time once its document closes; false removes it then. */
  readonly keep: boolean;
}

/** A copied file, read synchronously; both a RangeSource (Lamina PDF) and a RenderSource. */
export class OpfsByteSource implements RangeSource, RenderSource {
  readonly size: number;
  readonly #handle: SyncHandle;
  readonly #folder: FileSystemDirectoryHandle;
  readonly #name: string;
  readonly #keep: boolean;

  private constructor(handle: SyncHandle, folder: FileSystemDirectoryHandle, name: string, keep: boolean) {
    this.#handle = handle;
    this.#folder = folder;
    this.#name = name;
    this.#keep = keep;
    this.size = handle.getSize();
  }

  /**
   * `file` read from its OPFS copy, made now if it isn't there; `undefined` where the browser has
   * no OPFS or the copy can't be made (no room, locked by another tab, a write that fails), so the
   * caller reads it from memory. Rejects only when `options.signal` aborts.
   */
  static async open(file: File, options: CopyOptions): Promise<OpfsByteSource | undefined> {
    const folder = await cacheFolder();
    if (folder === undefined) return undefined;
    const name = copyName(file);
    let handle: SyncHandle;
    try {
      const fileHandle = (await folder.getFileHandle(name, { create: true })) as SyncFileHandle;
      handle = await fileHandle.createSyncAccessHandle();
    } catch {
      // Locked by another tab's worker, or the browser refuses: read it from memory instead.
      return undefined;
    }
    try {
      if (handle.getSize() !== file.size) {
        if (!(await makeRoom(folder, name, file.size))) throw new Error("no room for the copy");
        await copyInto(handle, file, options);
      }
      // The use record is best effort: another tab may hold it, and the copy is good without it.
      await touch(folder, name).catch(() => undefined);
      return new OpfsByteSource(handle, folder, name, options.keep);
    } catch (error) {
      // A copy cut short (a cancel, the disk full) is never left to look complete.
      handle.close();
      await removeCopy(folder, name);
      if (options.signal.aborted) throw error;
      return undefined;
    }
  }

  readSync(offset: number, length: number): Uint8Array | undefined {
    const end = Math.min(this.size, offset + length);
    if (offset < 0 || end < offset) return undefined;
    const out = new Uint8Array(end - offset);
    this.#handle.read(out, { at: offset });
    return out;
  }

  read(offset: number, length: number, options?: { readonly signal?: AbortSignal }): Promise<Uint8Array> {
    options?.signal?.throwIfAborted();
    return Promise.resolve(this.readSync(offset, length) ?? new Uint8Array());
  }

  /** Lets the copy go: kept in the folder for next time, or removed when it isn't to be kept. */
  close(): void {
    this.#handle.close();
    if (!this.#keep) void removeCopy(this.#folder, this.#name);
  }
}

async function cacheFolder(): Promise<FileSystemDirectoryHandle | undefined> {
  try {
    const root = await navigator.storage.getDirectory();
    return await root.getDirectoryHandle(CACHE_FOLDER, { create: true });
  } catch {
    return undefined;
  }
}

/** Removes a copy and its use record; a copy another tab is reading stays (false). */
async function removeCopy(folder: FileSystemDirectoryHandle, name: string): Promise<boolean> {
  try {
    await folder.removeEntry(name);
  } catch {
    return false;
  }
  await folder.removeEntry(name + USED).catch(() => undefined);
  return true;
}

/** Records that copy `name` was opened now. */
async function touch(folder: FileSystemDirectoryHandle, name: string): Promise<void> {
  const used = (await folder.getFileHandle(name + USED, { create: true })) as SyncFileHandle;
  const handle = await used.createSyncAccessHandle();
  try {
    const stamp = new Uint8Array(8);
    new DataView(stamp.buffer).setFloat64(0, Date.now());
    handle.truncate(0);
    handle.write(stamp, { at: 0 });
    handle.flush();
  } finally {
    handle.close();
  }
}

/** When copy `name` was last opened, else when it was made. */
async function lastUsed(folder: FileSystemDirectoryHandle, name: string, made: number): Promise<number> {
  try {
    const record = await (await folder.getFileHandle(name + USED)).getFile();
    return new DataView(await record.arrayBuffer()).getFloat64(0);
  } catch {
    return made;
  }
}

/**
 * Whether a copy of `size` bytes fits, making room by removing other copies, those used longest
 * ago first. The copies together may take half of what is free for them (the quota's free space
 * plus what the copies already hold), and never more than MAX_CACHE_BYTES.
 */
async function makeRoom(folder: FileSystemDirectoryHandle, keep: string, size: number): Promise<boolean> {
  const copies: { name: string; size: number; used: number }[] = [];
  for await (const [name, entry] of (
    folder as unknown as { entries(): AsyncIterable<[string, FileSystemHandle]> }
  ).entries()) {
    if (name === keep || name.endsWith(USED) || entry.kind !== "file") continue;
    const copy = await (entry as FileSystemFileHandle).getFile().catch(() => undefined);
    if (copy !== undefined)
      copies.push({ name, size: copy.size, used: await lastUsed(folder, name, copy.lastModified) });
  }
  let held = copies.reduce((sum, c) => sum + c.size, 0);
  const { quota = 0, usage = 0 } = await navigator.storage.estimate();
  const budget = Math.min(MAX_CACHE_BYTES, (quota - usage + held) / 2);
  if (size > budget) return false;
  for (const copy of copies.sort((a, b) => a.used - b.used)) {
    if (held + size <= budget) break;
    if (await removeCopy(folder, copy.name)) held -= copy.size;
  }
  return held + size <= budget;
}

async function copyInto(handle: SyncHandle, file: File, options: CopyOptions): Promise<void> {
  handle.truncate(0);
  for (let at = 0; at < file.size; at += CHUNK) {
    options.signal.throwIfAborted();
    const bytes = new Uint8Array(await file.slice(at, Math.min(file.size, at + CHUNK)).arrayBuffer());
    handle.write(bytes, { at });
    options.onProgress?.(Math.min(file.size, at + CHUNK), file.size);
  }
  handle.flush();
}
