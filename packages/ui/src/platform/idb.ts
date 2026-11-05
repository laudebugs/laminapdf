// A key-value store over one IndexedDB object store, for the web platform's small metadata
// (settings, recent documents, their file handles). Values are structured clones, so file
// handles survive a reload where the browser allows it.

/** Small values by key. */
export interface KeyValueStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

const DATABASE = "lamina-pdf";
const STORE = "kv";

/** A store in `factory`'s "lamina-pdf" database, opened on first use. */
export function createIdbStore(factory: IDBFactory): KeyValueStore {
  let database: Promise<IDBDatabase> | undefined;
  const open = () => {
    database ??= new Promise((resolve, reject) => {
      const request = factory.open(DATABASE, 1);
      request.onupgradeneeded = () => request.result.createObjectStore(STORE);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    return database;
  };
  const run = async <T>(mode: IDBTransactionMode, act: (store: IDBObjectStore) => IDBRequest<T>): Promise<T> => {
    const db = await open();
    return new Promise((resolve, reject) => {
      const request = act(db.transaction(STORE, mode).objectStore(STORE));
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  };
  return {
    get: (key) => run("readonly", (store) => store.get(key)),
    set: async (key, value) => void (await run("readwrite", (store) => store.put(value, key))),
    delete: async (key) => void (await run("readwrite", (store) => store.delete(key))),
    clear: async () => void (await run("readwrite", (store) => store.clear())),
  };
}
