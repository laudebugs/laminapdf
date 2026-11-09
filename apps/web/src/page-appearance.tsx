// The page appearance (R10, ADR 0024 §5): Original, Sepia, Dark or High contrast, apart from the
// UI theme. A choice is for this document or for all of them; this document's wins. With no choice
// at all, `forced-colors: active` picks High contrast, drawn in the system's CanvasText and Canvas.
// Choices are kept in localStorage: all documents' as a name, each local document's under a hash
// of its name and size (FNV-1a, so the key is there before the document's first render). A linked document's choice lasts until the tab closes: nothing about it is
// stored (ADR 0011). Choosing shows a toast with Undo.

import { PAGE_APPEARANCES, type PageAppearance } from "@laminapdf/tokens";
import {
  message,
  PageAppearanceFields,
  type PageAppearanceScope,
  type PlatformFile,
  pageAppearanceName,
  type ToastQueue,
  useMediaQuery,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { remoteUrlOf } from "./session.ts";

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

function isName(value: unknown): value is PageAppearance {
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

function readChoices(): PageAppearanceChoices {
  try {
    return parseChoices(localStorage.getItem(STORAGE_KEY));
  } catch {
    return { documents: {} };
  }
}

function saveChoices(choices: PageAppearanceChoices): void {
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

/** A system colour (`CanvasText`, `Canvas`) as the browser resolves it now. */
function systemColour(keyword: string): { r: number; g: number; b: number; a: number } {
  const probe = document.createElement("span");
  probe.style.color = keyword;
  document.body.append(probe);
  const [r = 0, g = 0, b = 0] = (getComputedStyle(probe).color.match(/\d+(\.\d+)?/g) ?? []).map(Number);
  probe.remove();
  return { r, g, b, a: 255 };
}

/** What the viewer draws for `name`: High contrast in the system's colours. */
function forViewer(name: PageAppearance): Parameters<Viewer["setAppearance"]>[0] {
  if (name !== "high-contrast") return name;
  return { kind: "high-contrast", ink: systemColour("CanvasText"), paper: systemColour("Canvas") };
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
type Choices = { stored: PageAppearanceChoices; session: PageAppearanceChoices };

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

/** What the hook gives the app. */
export interface PageAppearanceControls {
  /** The View settings popover's Page appearance section. */
  readonly section: ReactNode;
  /** `view.page-appearance` with its argument. */
  readonly actions: { readonly "view.page-appearance": (appearance: unknown) => void };
}

/** The open document's key, and whether its choice may be kept. */
function useDocumentPlace(file: PlatformFile | undefined): DocumentPlace {
  return useMemo(
    () => ({
      key: file === undefined ? undefined : documentKey(file),
      kept: file !== undefined && remoteUrlOf(file) === undefined,
    }),
    [file],
  );
}

/**
 * Bumped when the system's colours may have changed (a light or dark switch, the window coming
 * back into focus after a contrast theme change), so High contrast reads them again.
 */
function useSystemColourChanges(): number {
  const [changes, setChanges] = useState(0);
  useEffect(() => {
    const bump = () => setChanges((n) => n + 1);
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    scheme.addEventListener("change", bump);
    window.addEventListener("focus", bump);
    return () => {
      scheme.removeEventListener("change", bump);
      window.removeEventListener("focus", bump);
    };
  }, []);
  return changes;
}

/**
 * The page appearance of `viewer` (the open document's, `file`): resolved from the choices and
 * forced colours, applied to the viewer and to the page tokens (`data-page-appearance` on the root).
 */
export function usePageAppearance(
  viewer: Viewer | undefined,
  file: PlatformFile | undefined,
  toasts: ToastQueue,
): PageAppearanceControls {
  const [choices, setChoices] = useState(() => ({
    stored: readChoices(),
    session: { documents: {} } as PageAppearanceChoices,
  }));
  const [scope, setScope] = useState<PageAppearanceScope>("document");
  const forced = useMediaQuery("(forced-colors: active)");
  const doc = useDocumentPlace(file);
  const merged = { ...choices.stored, documents: { ...choices.stored.documents, ...choices.session.documents } };
  const name = resolve(merged, doc.key, forced);
  const system = useSystemColourChanges();
  // High contrast follows the system's colours: read again when they may have changed.
  const colours = name === "high-contrast" ? system : 0;
  useEffect(() => {
    document.documentElement.dataset.pageAppearance = name;
    if (colours >= 0) viewer?.setAppearance(forViewer(name));
  }, [viewer, name, colours]);
  // Undo reads the choices as they are when it is pressed, not when the toast showed.
  const latestChoices = useRef(choices);
  latestChoices.current = choices;
  const update = (next: typeof choices) => {
    latestChoices.current = next;
    setChoices(next);
    saveChoices(next.stored);
  };
  const pick = (appearance: PageAppearance, whom: PageAppearanceScope = scope) => {
    const before = choices;
    const after = choose(choices.stored, choices.session, doc, appearance, whom);
    update(after);
    const forDocument = whom === "document" && doc.key !== undefined;
    toasts.add({
      message: message(forDocument ? "pageAppearance.toast.document" : "pageAppearance.toast.all", {
        name: pageAppearanceName(appearance),
      }),
      action: {
        label: message("pageAppearance.undo"),
        onPress: () => update(undo(latestChoices.current, before, after, doc)),
      },
    });
  };
  // The command map is made once; it reaches the latest choices through this ref.
  const latest = useRef(pick);
  latest.current = pick;
  const actions = useMemo(
    () => ({ "view.page-appearance": (appearance: unknown) => isName(appearance) && latest.current(appearance) }),
    [],
  );
  const changeScope = (next: PageAppearanceScope) => {
    setScope(next);
    // Only a choice the user made moves; the default (Original, or High contrast under forced
    // colours) stays a default.
    const own = doc.key === undefined ? undefined : merged.documents[doc.key];
    if (doc.key !== undefined && (own !== undefined || choices.stored.all !== undefined)) {
      update(choose(choices.stored, choices.session, doc, name, next));
    }
  };
  return {
    section: (
      <PageAppearanceFields
        value={name}
        onChange={(appearance) => pick(appearance)}
        scope={file === undefined ? undefined : scope}
        onScopeChange={changeScope}
      />
    ),
    actions,
  };
}
