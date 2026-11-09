// The page appearance (R10, ADR 0024 §5): Original, Sepia, Dark or High contrast, apart from the
// UI theme. A choice is for this document or for all of them; this document's wins. With no choice
// at all, `forced-colors: active` picks High contrast, drawn in the system's CanvasText and Canvas.
// Choices are kept in localStorage: all documents' as a name, each local document's under a hash
// of its name and size. A linked document's choice lasts until the tab closes: nothing about it is
// stored (ADR 0011). Choosing shows a toast with Undo.

import type { PageAppearance } from "@laminapdf/tokens";
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

const NAMES: readonly PageAppearance[] = ["original", "sepia", "dark", "high-contrast"];

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

/** A document's key: a hash of its name and size, never its address. */
async function documentKey(file: PlatformFile): Promise<string> {
  const bytes = new TextEncoder().encode(`${file.name}\u0000${file.size}`);
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
  return Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
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

/** What the hook gives the app. */
export interface PageAppearanceControls {
  /** The View settings popover's Page appearance section. */
  readonly section: ReactNode;
  /** `view.page-appearance` with its argument. */
  readonly actions: { readonly "view.page-appearance": (appearance: unknown) => void };
}

/** The open document's key, and whether its choice may be kept. */
function useDocumentPlace(file: PlatformFile | undefined): DocumentPlace {
  const [key, setKey] = useState<string>();
  useEffect(() => {
    setKey(undefined);
    if (file === undefined) return;
    let current = true;
    void documentKey(file).then((k) => current && setKey(k), reportError);
    return () => {
      current = false;
    };
  }, [file]);
  return { key, kept: file !== undefined && remoteUrlOf(file) === undefined };
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
  useEffect(() => {
    document.documentElement.dataset.pageAppearance = name;
    viewer?.setAppearance(forViewer(name));
  }, [viewer, name]);
  const update = (next: typeof choices) => {
    setChoices(next);
    saveChoices(next.stored);
  };
  const pick = (appearance: PageAppearance, whom: PageAppearanceScope = scope) => {
    const before = choices;
    update(choose(choices.stored, choices.session, doc, appearance, whom));
    const forDocument = whom === "document" && doc.key !== undefined;
    toasts.add({
      message: message(forDocument ? "pageAppearance.toast.document" : "pageAppearance.toast.all", {
        name: pageAppearanceName(appearance),
      }),
      action: { label: message("pageAppearance.undo"), onPress: () => update(before) },
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
    if (doc.key !== undefined) update(choose(choices.stored, choices.session, doc, name, next));
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
