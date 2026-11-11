// The web platform's IndexedDB database, "lamina-pdf", and the key-value store in it for small
// metadata (settings, recent documents, their file handles). Values are structured clones, so file
// handles survive a reload where the browser allows it. Where there is no IndexedDB, or it can't
// open (private modes of some browsers, blocked site data), each store lives in memory.
//
// Versions: 1 has the `kv` store; 2 adds `positions` (reading positions by fingerprint, with an
// index on `readAt`; positions.ts).

/** Small values by key. */
export interface KeyValueStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

const DATABASE = "lamina-pdf";
const VERSION = 2;
const KV = "kv";
/** The reading positions' store and its index (positions.ts). */
export const POSITIONS = "positions";
export const BY_READ_AT = "readAt";

/** The database, opened on first call; `null` when it can't open, so its stores keep to memory. */
export type Database = () => Promise<IDBDatabase | null>;

/** `factory`'s "lamina-pdf" database at the current version; `null` throughout without a factory. */
export function openDatabase(factory: IDBFactory | undefined): Database {
  let database: Promise<IDBDatabase | null> | undefined;
  return () => {
    database ??= new Promise((resolve) => {
      if (factory === undefined) return resolve(null);
      try {
        let blocked = false;
        const request = factory.open(DATABASE, VERSION);
        request.onupgradeneeded = () => upgrade(request.result);
        request.onsuccess = () => {
          const db = request.result;
          // A newer tab upgrading the database waits on this one: close it, and this session's
          // later calls keep to memory instead of blocking that tab (a closed database would
          // refuse every transaction).
          db.onversionchange = () => {
            db.close();
            database = Promise.resolve(null);
          };
          // Opened once the older tab let go: later calls use the database (what was kept in memory
          // meanwhile stays there).
          if (blocked) database = Promise.resolve(db);
          else resolve(db);
        };
        request.onerror = () => resolve(null);
        // Another tab holds an older version open (one from before version 2 never closes for a
        // newer one): the session keeps its values in memory until that tab lets go.
        request.onblocked = () => {
          blocked = true;
          resolve(null);
        };
      } catch {
        resolve(null);
      }
    });
    return database;
  };
}

/** Creates the stores a database at an older version lacks. */
function upgrade(db: IDBDatabase): void {
  if (!db.objectStoreNames.contains(KV)) db.createObjectStore(KV);
  if (!db.objectStoreNames.contains(POSITIONS)) {
    db.createObjectStore(POSITIONS).createIndex(BY_READ_AT, BY_READ_AT);
  }
}

/** The key-value store in `database`; in memory when it doesn't open. */
export function createStore(database: Database): KeyValueStore {
  const memory = memoryStore();
  const store = async () => (await database()) ?? undefined;
  return {
    async get(key) {
      const db = await store();
      return db === undefined ? memory.get(key) : request(db, KV, "readonly", (s) => s.get(key));
    },
    async set(key, value) {
      const db = await store();
      if (db === undefined) return memory.set(key, value);
      await request(db, KV, "readwrite", (s) => s.put(value, key));
    },
    async delete(key) {
      const db = await store();
      if (db === undefined) return memory.delete(key);
      await request(db, KV, "readwrite", (s) => s.delete(key));
    },
    async clear() {
      const db = await store();
      if (db === undefined) return memory.clear();
      await request(db, KV, "readwrite", (s) => s.clear());
    },
  };
}

/**
 * `act`'s result on store `name`, once its transaction completes: a write the browser aborts later
 * (out of quota) rejects.
 */
export function request<T>(
  db: IDBDatabase,
  name: string,
  mode: IDBTransactionMode,
  act: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const transaction = db.transaction(name, mode);
    const pending = act(transaction.objectStore(name));
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
