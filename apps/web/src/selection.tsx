// The selection surfaces (UI/UX plan §5.3, ADR 0024): with a mouse or a pen, a toolbar by the
// selected text once the pointer is up (Copy, Copy exact, Search for), hidden while a drag or a
// scroll goes on, Shift+F10 to reach it and Escape to dismiss it, and a command to turn it off;
// with touch, the selection's bar in the phone's bottom bar instead, so the OS's own menu by the
// text stays uncovered. The viewer stays headless: it reports the selection and copies; the
// commands it runs are the registry's, so the palette and the shortcuts reach them too.

import {
  Button,
  type CommandId,
  type KeyPlatform,
  type MoreItems,
  message,
  type PlatformStorage,
  SelectionActionBar,
  SelectionToolbar,
  selectionBox,
  useMediaQuery,
} from "@laminapdf/ui";
import type { TextSelection, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import type { Find } from "./find.tsx";

/** Where the toolbar setting is kept. */
const TOOLBAR_KEY = "selection.toolbar";

/** The selection's More entries on a phone. */
type SelectionMoreId = "copy-exact";

/** The selection's command handlers and surfaces. */
export interface SelectionSurfaces {
  readonly actions: Partial<Record<CommandId, () => void>>;
  /** The toolbar, while a settled mouse or pen selection shows it (it places itself). */
  readonly toolbar: ReactNode | undefined;
  /** The phone's selection bar, while text is selected by touch; the bottom bar otherwise. */
  readonly bar: ReactNode | undefined;
}

/** Whether the toolbar is on: the setting, read once and kept on change. */
function useToolbarSetting(storage: PlatformStorage): [boolean, () => void] {
  const [on, setOn] = useState(true);
  useEffect(() => {
    let live = true;
    storage
      .get(TOOLBAR_KEY)
      .then((value) => {
        if (live && typeof value === "boolean") setOn(value);
      })
      .catch(() => {
        // Nothing kept, or no storage: the toolbar stays on.
      });
    return () => {
      live = false;
    };
  }, [storage]);
  const toggle = () => {
    setOn((was) => {
      void storage.set(TOOLBAR_KEY, !was).catch(() => undefined);
      return !was;
    });
  };
  return [on, toggle];
}

/** The text selection, as the viewer last reported it; hidden while the view scrolls. */
function useViewerSelection(viewer: Viewer | undefined): [TextSelection | undefined, boolean] {
  const [selection, setSelection] = useState<TextSelection | undefined>(undefined);
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    setSelection(undefined);
    if (viewer === undefined) return;
    const off = viewer.on("selectionchange", ({ selection: next }) => {
      setSelection(next);
      // The pointer is up again: a toolbar a scroll hid may show.
      if (next?.settled === true) setScrolled(false);
    });
    // A scroll hides the toolbar until the pointer next settles a selection.
    const onScroll = () => setScrolled(true);
    viewer.element.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      off();
      viewer.element.removeEventListener("scroll", onScroll);
    };
  }, [viewer]);
  return [selection, scrolled];
}

/** The selection's handlers, its desktop toolbar and its phone bar. */
export function useSelection(
  viewer: Viewer | undefined,
  find: Find,
  storage: PlatformStorage,
  keys: KeyPlatform,
): SelectionSurfaces {
  const [selection, scrolled] = useViewerSelection(viewer);
  const [toolbarOn, toggleToolbar] = useToolbarSetting(storage);
  const [dismissed, setDismissed] = useState<TextSelection | undefined>(undefined);
  const coarse = useMediaQuery("(pointer: coarse)");
  const share = typeof navigator.share === "function";
  const actions = useMemo<Partial<Record<CommandId, () => void>>>(() => {
    if (viewer === undefined) return {};
    const text = () => selection?.text ?? "";
    return {
      "edit.copy": () => void viewer.copySelection("paragraph"),
      "edit.copy-exact": () => void viewer.copySelection("exact"),
      "edit.select-column-mode": () => {
        viewer.columnSelect = !viewer.columnSelect;
      },
      "selection.search": () => find.searchFor(text()),
      "selection.toggle-toolbar": toggleToolbar,
      ...(share ? { "selection.share": () => void navigator.share({ text: text() }).catch(() => undefined) } : {}),
    };
  }, [viewer, selection, find, toggleToolbar, share]);
  const touch =
    selection !== undefined && (selection.pointerType === "touch" || (selection.pointerType === "" && coarse));
  const showToolbar =
    viewer !== undefined && toolbarOn && selection?.settled === true && !touch && !scrolled && dismissed !== selection;
  const more: MoreItems<SelectionMoreId> = {
    items: [{ id: "copy-exact", label: message("command.edit.copy-exact"), icon: "copy" }],
    onAction: () => actions["edit.copy-exact"]?.(),
  };
  return {
    actions,
    toolbar: showToolbar ? (
      <SelectionToolbar
        aria-label={message("selection.toolbar")}
        anchor={() => selectionBox(selection.rects)}
        bounds={() => viewerBounds(viewer)}
        onDismiss={() => setDismissed(selection)}
      >
        <Button size={32} icon="copy" preventFocusOnPress onPress={() => actions["edit.copy"]?.()}>
          {message("command.edit.copy")}
        </Button>
        <Button size={32} preventFocusOnPress onPress={() => actions["edit.copy-exact"]?.()}>
          {message("command.edit.copy-exact")}
        </Button>
        <Button size={32} icon="find" preventFocusOnPress onPress={() => actions["selection.search"]?.()}>
          {message("selection.search")}
        </Button>
      </SelectionToolbar>
    ) : undefined,
    bar: touch ? <SelectionActionBar more={more} keys={keys} /> : undefined,
  };
}

/** The viewer's box on screen: the toolbar stays inside it. */
function viewerBounds(viewer: Viewer): { top: number; left: number; width: number; height: number } {
  const r = viewer.element.getBoundingClientRect();
  return { top: r.top, left: r.left, width: r.width, height: r.height };
}
