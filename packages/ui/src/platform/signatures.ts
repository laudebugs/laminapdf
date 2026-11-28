// The web platform's signature library (ADR 0010 §1, `Platform.signatures`; E10c): the pictures of
// a handwritten signature a user keeps on this device, in the `signatures` store of the
// "lamina-pdf" database, newest first. The desktop webview keeps the same database in the app data
// folder. Where the database doesn't open, the signatures live in memory for the session, as the
// key-value store's do. Nothing here is a certificate or a key, and nothing leaves the device.

import { type Database, request, SIGNATURES } from "./idb.js";
import type { SavedSignature, SignatureLibrary } from "./types.js";

/** How many signatures are kept: a person has a handful, and each is a small picture. */
export const SIGNATURES_LIMIT = 50;

/** The signatures in `database`'s `signatures` store; in memory when it doesn't open. */
export function createSignatureLibrary(database: Database, limit = SIGNATURES_LIMIT): SignatureLibrary {
  const memory = memoryLibrary(limit);
  const store = async () => (await database()) ?? undefined;
  return {
    async list() {
      const db = await store();
      if (db === undefined) return memory.list();
      const all = await request(db, SIGNATURES, "readonly", (s) => s.getAll() as IDBRequest<unknown[]>);
      return newestFirst(all.filter(isSignature));
    },
    async put(signature) {
      const db = await store();
      if (db === undefined) return memory.put(signature);
      await request(db, SIGNATURES, "readwrite", (s) => {
        const put = s.put(signature);
        put.onsuccess = () => trim(s, limit);
        return put;
      });
    },
    async remove(id) {
      const db = await store();
      if (db === undefined) return memory.remove(id);
      await request(db, SIGNATURES, "readwrite", (s) => s.delete(id));
    },
    async clear() {
      const db = await store();
      if (db === undefined) return memory.clear();
      await request(db, SIGNATURES, "readwrite", (s) => s.clear());
    },
  };
}

function newestFirst(signatures: SavedSignature[]): SavedSignature[] {
  return signatures.sort((a, b) => b.added - a.added);
}

/** Deletes the oldest signatures until `store` holds `limit`, in the caller's transaction. */
function trim(store: IDBObjectStore, limit: number): void {
  const all = store.getAll() as IDBRequest<unknown[]>;
  all.onsuccess = () => {
    const kept = newestFirst(all.result.filter(isSignature));
    for (const old of kept.slice(limit)) store.delete(old.id);
  };
}

/** Whether `value` has the shape a signature is stored in (a store another version wrote may not). */
function isSignature(value: unknown): value is SavedSignature {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Partial<SavedSignature>;
  return (
    typeof v.id === "string" &&
    typeof v.name === "string" &&
    v.bytes instanceof Uint8Array &&
    typeof v.width === "number" &&
    typeof v.height === "number" &&
    typeof v.added === "number"
  );
}

function memoryLibrary(limit: number): SignatureLibrary {
  const map = new Map<string, SavedSignature>();
  return {
    list: async () => newestFirst([...map.values()]),
    async put(signature) {
      map.set(signature.id, signature);
      for (const old of newestFirst([...map.values()]).slice(limit)) map.delete(old.id);
    },
    remove: async (id) => void map.delete(id),
    clear: async () => map.clear(),
  };
}
