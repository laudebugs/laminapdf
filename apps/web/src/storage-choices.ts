// What the user chose to keep in this browser (Settings › Storage and privacy): whether recent
// local files are listed, which also decides whether a large file's disk copy outlives its
// document, and whether each document's reading position is kept, with the last document to come
// back to. Kept in localStorage, read by the recent list and by the engine's opens. A linked file
// is never listed: ADR 0011 keeps a document's address out of storage and the recent list.

/** Where the choices are kept, in this browser only. */
const STORAGE_KEY = "lamina.storage";

/** What the user chose to remember. */
export interface StorageChoices {
  /** List local files on the home screen, and keep large files' copies for next time (default on). */
  readonly rememberLocal: boolean;
  /** Keep where the reader stopped in each document, and reopen it there (default on). */
  readonly rememberPositions: boolean;
}

const DEFAULT_CHOICES: StorageChoices = { rememberLocal: true, rememberPositions: true };

/** The stored choices, or the defaults where storage is blocked, empty or damaged. */
export function parseChoices(stored: string | null): StorageChoices {
  try {
    const value = JSON.parse(stored ?? "{}") as Partial<Record<keyof StorageChoices, unknown>>;
    const choice = (key: keyof StorageChoices) => {
      const stored = value[key];
      return typeof stored === "boolean" ? stored : DEFAULT_CHOICES[key];
    };
    return { rememberLocal: choice("rememberLocal"), rememberPositions: choice("rememberPositions") };
  } catch {
    return DEFAULT_CHOICES;
  }
}

/** The choices made in this tab, which outlast a save that storage refused. */
let current: StorageChoices | undefined;

/** The choices now. */
export function readChoices(): StorageChoices {
  if (current !== undefined) return current;
  try {
    return parseChoices(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_CHOICES;
  }
}

/** Keeps `choices`; with storage blocked they last until the tab closes. */
export function saveChoices(choices: StorageChoices): void {
  current = choices;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choices));
  } catch {
    // Storage blocked: the choice lasts until the tab closes.
  }
}
