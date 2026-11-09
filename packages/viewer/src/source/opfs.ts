// A large local file read from the origin-private file system (C7b): copied there once, in the
// document worker, then read through a synchronous access handle, so neither Lamina PDF nor the
// render backend (PDFium reads synchronously) needs the whole file in memory. Copies live in one
// folder, named by the file's size, date and name, so opening the same file again reads the copy
// already there; the oldest copies go when the folder would pass its budget. OPFS is a cache,
// never the only copy: a browser without it, a copy that doesn't fit the quota or a file locked by
// another tab opens from memory, as before.

import type { RenderSource } from "@laminapdf/render";
import type { RangeSource } from "@laminapdf/sdk";

/** The OPFS folder of document copies; the app clears the same folder (apps/web document-cache.ts). */
export const CACHE_FOLDER = "documents";

/** Files above this many bytes are read from a copy: 200 MiB. */
export const COPY_ABOVE = 200 * 1024 * 1024;

/** How much the copies may take, at most: 2 GiB, and never more than half the free quota. */
const MAX_CACHE_BYTES = 2 * 1024 * 1024 * 1024;

/** Copied in steps of 8 MiB, each a chance to stop and to report progress. */
const CHUNK = 8 * 1024 * 1024;

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

/** A copied file, read synchronously; both a RangeSource (Lamina PDF) and a RenderSource. */
export class OpfsByteSource implements RangeSource, RenderSource {
  readonly size: number;
  readonly #handle: SyncHandle;

  private constructor(handle: SyncHandle) {
    this.#handle = handle;
    this.size = handle.getSize();
  }

  /**
   * `file` read from its OPFS copy, made now if it isn't there (`onProgress` is told each step);
   * `undefined` where the browser has no OPFS, the copy wouldn't fit, or the file is locked.
   */
  static async open(
    file: File,
    signal: AbortSignal,
    onProgress?: (loaded: number, total: number) => void,
  ): Promise<OpfsByteSource | undefined> {
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
        if (!(await makeRoom(folder, name, file.size))) {
          handle.close();
          await folder.removeEntry(name).catch(() => undefined);
          return undefined;
        }
        await copyInto(handle, file, signal, onProgress);
      }
      return new OpfsByteSource(handle);
    } catch (error) {
      // A copy cut short (a cancel, the disk full) is never left to look complete.
      handle.close();
      await folder.removeEntry(name).catch(() => undefined);
      throw error;
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

  /** Lets the copy go (it stays in the folder for next time). */
  close(): void {
    this.#handle.close();
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

/**
 * Whether a copy of `size` bytes fits, making room by removing older copies, oldest first. The
 * copies together may take half of what is free for them (the quota's free space plus what the
 * copies already hold), and never more than MAX_CACHE_BYTES.
 */
async function makeRoom(folder: FileSystemDirectoryHandle, keep: string, size: number): Promise<boolean> {
  const copies: { name: string; size: number; date: number }[] = [];
  for await (const [name, entry] of (
    folder as unknown as { entries(): AsyncIterable<[string, FileSystemHandle]> }
  ).entries()) {
    if (name === keep || entry.kind !== "file") continue;
    const copy = await (entry as FileSystemFileHandle).getFile().catch(() => undefined);
    if (copy !== undefined) copies.push({ name, size: copy.size, date: copy.lastModified });
  }
  let used = copies.reduce((sum, c) => sum + c.size, 0);
  const { quota = 0, usage = 0 } = await navigator.storage.estimate();
  const budget = Math.min(MAX_CACHE_BYTES, (quota - usage + used) / 2);
  if (size > budget) return false;
  for (const copy of copies.sort((a, b) => a.date - b.date)) {
    if (used + size <= budget) break;
    // A copy another tab is reading stays; the space it holds still counts.
    if (
      await folder.removeEntry(copy.name).then(
        () => true,
        () => false,
      )
    )
      used -= copy.size;
  }
  return used + size <= budget;
}

async function copyInto(
  handle: SyncHandle,
  file: File,
  signal: AbortSignal,
  onProgress?: (loaded: number, total: number) => void,
): Promise<void> {
  handle.truncate(0);
  for (let at = 0; at < file.size; at += CHUNK) {
    signal.throwIfAborted();
    const bytes = new Uint8Array(await file.slice(at, Math.min(file.size, at + CHUNK)).arrayBuffer());
    handle.write(bytes, { at });
    onProgress?.(Math.min(file.size, at + CHUNK), file.size);
  }
  handle.flush();
}
