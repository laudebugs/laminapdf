// Find in the reader (R05): the find bar in the secondary-toolbar slot while find is open, and the
// registry's find commands on the viewer's find controller. Ctrl+F or ⌘F (`find.open`) opens the
// bar, or puts the focus back in its field; Close or Escape closes it and clears the highlights;
// another document closes it too.

import { FindBar } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState } from "react";

/** The handlers of the registry's find commands. */
export type FindActions = Readonly<Record<"find.open" | "find.next" | "find.previous", () => void>>;

/** Find's commands, and a search for given text (the selection's `Search for`). */
export interface Find {
  readonly actions: FindActions;
  readonly bar: ReactNode | undefined;
  readonly open: boolean;
  /** Opens find searching for `text`, the bar's field holding it. */
  readonly searchFor: (text: string) => void;
}

/** The find commands' handlers, the bar while find is open, whether it is, and a search from outside the bar. */
export function useFind(viewer: Viewer | undefined, file: object | undefined): Find {
  const [open, setOpen] = useState(false);
  // A new bar for each search started from outside it, so its field starts with that search's text.
  const [searches, setSearches] = useState(0);
  // Counts `find.open` while the bar is open, which puts the focus back in its field.
  const [focusRequest, setFocusRequest] = useState(0);
  // Another document, or none: find closes.
  useEffect(() => {
    setOpen(false);
    viewer?.find.close();
  }, [file]);
  const actions = useMemo(
    () => ({
      "find.open": () => {
        if (!open) setOpen(true);
        else setFocusRequest((n) => n + 1);
      },
      "find.next": () => viewer?.find.next(),
      "find.previous": () => viewer?.find.previous(),
    }),
    [viewer, open],
  );
  const close = () => {
    viewer?.find.close();
    setOpen(false);
  };
  const bar =
    open && viewer !== undefined ? (
      <FindBar key={searches} find={viewer.find} onClose={close} focusRequest={focusRequest} />
    ) : undefined;
  const searchFor = (text: string) => {
    const query = text.replace(/\s+/g, " ").trim();
    if (viewer === undefined || query === "") return;
    viewer.find.find(query, viewer.find.getState().options);
    setSearches((n) => n + 1);
    setOpen(true);
  };
  return { actions, bar, open: bar !== undefined, searchFor };
}
