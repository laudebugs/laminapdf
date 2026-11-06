// A key-value store over one IndexedDB object store, for the web platform's small metadata
// (settings, recent documents, their file handles). Values are structured clones, so file
// handles survive a reload where the browser allows it. Where there is no IndexedDB, or it
// can't open (private modes of some browsers, blocked site data), the store lives in memory.

/** Small values by key. */
export interface KeyValueStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

const DATABASE = "lamina-pdf";
const STORE = "kv";

/** A store in `factory`'s "lamina-pdf" database, opened on first use; in memory without one. */
export function createStore(factory: IDBFactory | undefined): KeyValueStore {
  const memory = memoryStore();
  if (factory === undefined) return memory;
  // Opened once: null when it can't open, and the memory store serves the session instead.
  let database: Promise<IDBDatabase | null> | undefined;
  const open = () => {
    database ??= new Promise((resolve) => {
      try {
        const request = factory.open(DATABASE, 1);
        request.onupgradeneeded = () => request.result.createObjectStore(STORE);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => resolve(null);
        // Another tab holds an older version open: the session keeps its values in memory.
        request.onblocked = () => resolve(null);
      } catch {
        resolve(null);
      }
    });
    return database;
  };
  const store = async () => (await open()) ?? undefined;
  return {
    async get(key) {
      const db = await store();
      return db === undefined ? memory.get(key) : request(db, "readonly", (s) => s.get(key));
    },
    async set(key, value) {
      const db = await store();
      if (db === undefined) return memory.set(key, value);
      await request(db, "readwrite", (s) => s.put(value, key));
    },
    async delete(key) {
      const db = await store();
      if (db === undefined) return memory.delete(key);
      await request(db, "readwrite", (s) => s.delete(key));
    },
    async clear() {
      const db = await store();
      if (db === undefined) return memory.clear();
      await request(db, "readwrite", (s) => s.clear());
    },
  };
}

/** `act`'s result, once its transaction completes: a write the browser aborts later (out of quota) rejects. */
function request<T>(
  db: IDBDatabase,
  mode: IDBTransactionMode,
  act: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(STORE, mode);
    const pending = act(transaction.objectStore(STORE));
    transaction.oncomplete = () => resolve(pending.result);
    transaction.onerror = () => reject(transaction.error ?? pending.error);
    transaction.onabort = () => reject(transaction.error ?? pending.error);
  });
}

function memoryStore(): KeyValueStore {
  const map = new Map<string, unknown>();
  return {
    get: async (key) => map.get(key),
    set: async (key, value) => void map.set(key, value),
    delete: async (key) => void map.delete(key),
    clear: async () => map.clear(),
  };
}
