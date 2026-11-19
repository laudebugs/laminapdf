// The custom stamps kept on this device (Paper C05): PNG and JPEG images, each with a name, kept
// in IndexedDB on the web (E5 plan §6) and listed newest first. The library checks an image's
// bytes, not its name or type, so only a real PNG or JPEG becomes a stamp. The storage is a small
// interface, so the library runs in Node over a map.

/** A custom stamp. */
export interface CustomStamp {
  readonly id: string;
  readonly name: string;
  /** `image/png` or `image/jpeg`, from the bytes. */
  readonly type: "image/png" | "image/jpeg";
  readonly bytes: Uint8Array;
  /** When it was added, ms since the epoch: the list's order. */
  readonly added: number;
}

/** Where the stamps are kept: IndexedDB in a browser, a map in tests. */
export interface StampStore {
  all(): Promise<CustomStamp[]>;
  put(stamp: CustomStamp): Promise<void>;
  delete(id: string): Promise<void>;
}

/** The PNG and JPEG signatures. */
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG = [0xff, 0xd8, 0xff];

/** `bytes`' image type, from their signature; undefined for anything but PNG and JPEG. */
export function imageType(bytes: Uint8Array): CustomStamp["type"] | undefined {
  const starts = (signature: readonly number[]) => signature.every((b, i) => bytes[i] === b);
  if (starts(PNG)) return "image/png";
  if (starts(JPEG)) return "image/jpeg";
  return undefined;
}

/** A file name without its extension, as a stamp's first name: "signature.png" is "signature". */
export function stampName(fileName: string): string {
  const dot = fileName.lastIndexOf(".");
  const base = dot > 0 ? fileName.slice(0, dot) : fileName;
  return base.trim() === "" ? fileName : base.trim();
}

/** The custom stamp library over `store`. */
export class StampLibrary {
  readonly #store: StampStore;
  readonly #now: () => number;
  readonly #id: () => string;

  constructor(store: StampStore, now: () => number = Date.now, id: () => string = () => crypto.randomUUID()) {
    this.#store = store;
    this.#now = now;
    this.#id = id;
  }

  /** Every stamp, newest first. */
  async list(): Promise<CustomStamp[]> {
    return (await this.#store.all()).sort((a, b) => b.added - a.added);
  }

  /** One stamp, by its ID. */
  async get(id: string): Promise<CustomStamp | undefined> {
    return (await this.#store.all()).find((s) => s.id === id);
  }

  /** Adds the image `bytes` of file `fileName`; undefined, and nothing kept, when they aren't a PNG or JPEG. */
  async add(fileName: string, bytes: Uint8Array): Promise<CustomStamp | undefined> {
    const type = imageType(bytes);
    if (type === undefined) return undefined;
    const stamp: CustomStamp = { id: this.#id(), name: stampName(fileName), type, bytes, added: this.#now() };
    await this.#store.put(stamp);
    return stamp;
  }

  /** Renames stamp `id`; a blank name leaves it as it was. */
  async rename(id: string, name: string): Promise<void> {
    const stamp = await this.get(id);
    if (stamp === undefined || name.trim() === "") return;
    await this.#store.put({ ...stamp, name: name.trim() });
  }

  async remove(id: string): Promise<void> {
    await this.#store.delete(id);
  }
}

/** The IndexedDB database and store the stamps are kept in. */
const DATABASE = "lamina-stamps";
const STORE = "custom";

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error);
  });
}

/** The stamps in this browser's IndexedDB, opened on first use. */
export function indexedDbStore(): StampStore {
  let opened: Promise<IDBDatabase> | undefined;
  const database = () => {
    opened ??= new Promise<IDBDatabase>((resolve, reject) => {
      const open = indexedDB.open(DATABASE, 1);
      open.onupgradeneeded = () => open.result.createObjectStore(STORE, { keyPath: "id" });
      open.onsuccess = () => resolve(open.result);
      open.onerror = () => {
        opened = undefined;
        reject(open.error);
      };
    });
    return opened;
  };
  const store = async (mode: IDBTransactionMode) => (await database()).transaction(STORE, mode).objectStore(STORE);
  return {
    all: async () => request((await store("readonly")).getAll() as IDBRequest<CustomStamp[]>),
    put: async (stamp) => {
      await request((await store("readwrite")).put(stamp));
    },
    delete: async (id) => {
      await request((await store("readwrite")).delete(id));
    },
  };
}
