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
  usePageAppearanceValue,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useMemo, useRef, useState } from "react";
import { remoteUrlOf } from "./open/remote-file.ts";
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
  usePageAppearanceValue(viewer, name);
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
