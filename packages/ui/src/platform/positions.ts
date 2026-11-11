// The web platform's reading positions (ADR 0010 §1, `Platform.positions`): one entry per document
// fingerprint, in the `positions` store of the "lamina-pdf" database, trimmed to the newest 500
// in the same transaction as each write, so two tabs writing at once can't leave more. Where the
// database doesn't open, the positions live in memory for the session, as the key-value store's do.

import { BY_READ_AT, type Database, POSITIONS, request } from "./idb.js";
import type { ReadingPosition, ReadingPositions } from "./types.js";

/** How many documents' positions are kept; the one read longest ago goes first. */
export const POSITIONS_LIMIT = 500;

/** Reading positions in `database`'s `positions` store; in memory when it doesn't open. */
export function createPositions(database: Database, limit = POSITIONS_LIMIT): ReadingPositions {
  const memory = memoryPositions(limit);
  const store = async () => (await database()) ?? undefined;
  return {
    async get(fingerprint) {
      const db = await store();
      if (db === undefined) return memory.get(fingerprint);
      return asPosition(await request(db, POSITIONS, "readonly", (s) => s.get(fingerprint)));
    },
    async set(fingerprint, entry) {
      const db = await store();
      if (db === undefined) return memory.set(fingerprint, entry);
      const value: ReadingPosition = { view: entry.view, pageCount: entry.pageCount, readAt: entry.readAt };
      await request(db, POSITIONS, "readwrite", (s) => {
        const put = s.put(value, fingerprint);
        put.onsuccess = () => trim(s, limit);
        return put;
      });
    },
    async remove(fingerprint) {
      const db = await store();
      if (db === undefined) return memory.remove(fingerprint);
      await request(db, POSITIONS, "readwrite", (s) => s.delete(fingerprint));
    },
    async clear() {
      const db = await store();
      if (db === undefined) return memory.clear();
      await request(db, POSITIONS, "readwrite", (s) => s.clear());
    },
  };
}

/** Deletes the entries read longest ago until `store` holds `limit`, in the caller's transaction. */
function trim(store: IDBObjectStore, limit: number): void {
  const count = store.count();
  count.onsuccess = () => {
    let excess = count.result - limit;
    if (excess <= 0) return;
    const cursor = store.index(BY_READ_AT).openCursor();
    cursor.onsuccess = () => {
      const at = cursor.result;
      if (at === null || excess <= 0) return;
      at.delete();
      excess -= 1;
      at.continue();
    };
  };
}

/** `value` as a position when it has the shape one is stored in, else undefined. */
function asPosition(value: unknown): ReadingPosition | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const { view, pageCount, readAt } = value as Partial<ReadingPosition>;
  if (typeof pageCount !== "number" || typeof readAt !== "number") return undefined;
  return { view, pageCount, readAt };
}

function memoryPositions(limit: number): ReadingPositions {
  const map = new Map<string, ReadingPosition>();
  return {
    get: async (fingerprint) => map.get(fingerprint),
    async set(fingerprint, entry) {
      map.delete(fingerprint);
      map.set(fingerprint, entry);
      const oldest = [...map].sort(([, a], [, b]) => a.readAt - b.readAt);
      for (const [key] of oldest.slice(0, Math.max(0, map.size - limit))) map.delete(key);
    },
    remove: async (fingerprint) => void map.delete(fingerprint),
    clear: async () => map.clear(),
  };
}
