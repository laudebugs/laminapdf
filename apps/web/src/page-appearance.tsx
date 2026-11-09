// The page appearance (R10, ADR 0024 §5): Original, Sepia, Dark or High contrast, apart from the
// UI theme. A choice is for this document or for all of them; this document's wins. With no choice
// at all, `forced-colors: active` picks High contrast, drawn in the system's CanvasText and Canvas.
// Choices are kept in localStorage (page-appearance-choices.ts): all documents' as a name, each
// local document's under a hash of its name and size. A linked document's choice lasts until the
// tab closes: nothing about it is stored (ADR 0011). Choosing shows a toast with Undo.

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
import {
  choose,
  type DocumentPlace,
  documentKey,
  isName,
  type PageAppearanceChoices,
  readChoices,
  resolve,
  saveChoices,
  undo,
} from "./page-appearance-choices.ts";
import { remoteUrlOf } from "./session.ts";

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
