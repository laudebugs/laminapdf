// The page appearance choices (R10): what the user chose for all documents and for each one, how
// they are read, stored and resolved, and what choosing or undoing does to them. Pure but for
// localStorage, so the reader's tests run without the viewer; page-appearance.tsx is the hook.

import { PAGE_APPEARANCES, type PageAppearance } from "@laminapdf/tokens";
import type { PageAppearanceScope, PlatformFile } from "@laminapdf/ui";

/** Where the choices are kept, in this browser only. */
const STORAGE_KEY = "lamina.page-appearance";

/** At most this many documents keep their own appearance; the oldest go first. */
const MAX_DOCUMENTS = 200;

const NAMES = Object.keys(PAGE_APPEARANCES) as readonly PageAppearance[];

/** What the user chose: for all documents, and for some documents by key. */
export interface PageAppearanceChoices {
  readonly all?: PageAppearance | undefined;
  readonly documents: Readonly<Record<string, PageAppearance>>;
}

export function isName(value: unknown): value is PageAppearance {
  return NAMES.includes(value as PageAppearance);
}

/** The stored choices; none where storage is blocked, empty or damaged. */
export function parseChoices(stored: string | null): PageAppearanceChoices {
  try {
    const value = JSON.parse(stored ?? "{}") as { all?: unknown; documents?: unknown };
    const documents = Object.fromEntries(
      Object.entries(typeof value.documents === "object" && value.documents !== null ? value.documents : {}).filter(
        (entry): entry is [string, PageAppearance] => isName(entry[1]),
      ),
    );
    return { ...(isName(value.all) ? { all: value.all } : {}), documents };
  } catch {
    return { documents: {} };
  }
}

export function readChoices(): PageAppearanceChoices {
  try {
    return parseChoices(localStorage.getItem(STORAGE_KEY));
  } catch {
    return { documents: {} };
  }
}

export function saveChoices(choices: PageAppearanceChoices): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(choices));
  } catch {
    // Storage blocked: the choice lasts until the tab closes.
  }
}

/** `choices` with `key`'s appearance set (`undefined` removes it), keeping the newest few. */
export function withDocument(
  choices: PageAppearanceChoices,
  key: string,
  appearance: PageAppearance | undefined,
): PageAppearanceChoices {
  const { [key]: _, ...others } = choices.documents;
  const entries = Object.entries(others);
  const kept = appearance === undefined ? entries : [...entries, [key, appearance] as const];
  return { ...choices, documents: Object.fromEntries(kept.slice(-MAX_DOCUMENTS)) };
}

/** The appearance shown: this document's, else all documents', else High contrast under forced colours. */
export function resolve(choices: PageAppearanceChoices, key: string | undefined, forced: boolean): PageAppearance {
  return (
    (key === undefined ? undefined : choices.documents[key]) ?? choices.all ?? (forced ? "high-contrast" : "original")
  );
}

/**
 * A document's key: a 64-bit FNV-1a hash of its name and size (two 32-bit halves with different
 * offsets), never its address. Synchronous, so the document's own appearance is known before its
 * first render.
 */
export function documentKey(file: Pick<PlatformFile, "name" | "size">): string {
  const text = `${file.name}\u0000${file.size}`;
  let low = 0x811c9dc5;
  let high = 0x050c5d1f;
  for (let i = 0; i < text.length; i++) {
    const c = text.charCodeAt(i);
    low = Math.imul(low ^ c, 0x01000193) >>> 0;
    high = Math.imul(high ^ c, 0x01000193) >>> 0;
  }
  return high.toString(16).padStart(8, "0") + low.toString(16).padStart(8, "0");
}

/** Where the open document's choices go: kept (a local file) or for this tab only (a link). */
export interface DocumentPlace {
  readonly key: string | undefined;
  readonly kept: boolean;
}

/** The stored and this tab's choices after choosing `appearance` for this document or for all. */
export function choose(
  stored: PageAppearanceChoices,
  session: PageAppearanceChoices,
  doc: DocumentPlace,
  appearance: PageAppearance,
  whom: PageAppearanceScope,
): { stored: PageAppearanceChoices; session: PageAppearanceChoices } {
  const { key, kept } = doc;
  if (key === undefined) return { stored: { ...stored, all: appearance }, session };
  if (whom === "document") {
    return kept
      ? { stored: withDocument(stored, key, appearance), session }
      : { stored, session: withDocument(session, key, appearance) };
  }
  // For all documents: this one's own choice goes, so it follows too.
  return {
    stored: { ...(kept ? withDocument(stored, key, undefined) : stored), all: appearance },
    session: kept ? session : withDocument(session, key, undefined),
  };
}

/** The stored and this tab's choices. */
export type Choices = { stored: PageAppearanceChoices; session: PageAppearanceChoices };

/**
 * The choices after undoing one: `before` and `after` are the choices around it. Each slot it
 * changed (this document's own choice; for all documents also theirs) goes back to `before`'s
 * value, unless a later choice changed that slot again.
 */
export function undo(now: Choices, before: Choices, after: Choices, doc: DocumentPlace): Choices {
  let { stored, session } = now;
  const { key } = doc;
  if (key !== undefined) {
    if (stored.documents[key] === after.stored.documents[key])
      stored = withDocument(stored, key, before.stored.documents[key]);
    if (session.documents[key] === after.session.documents[key]) {
      session = withDocument(session, key, before.session.documents[key]);
    }
  }
  if (stored.all === after.stored.all) stored = { ...stored, all: before.stored.all };
  return { stored, session };
}
