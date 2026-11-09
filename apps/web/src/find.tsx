// Find in the reader (R05): the find bar in the secondary-toolbar slot while find is open, and the
// registry's find commands on the viewer's find controller. Ctrl+F or ⌘F (`find.open`) opens the
// bar, or puts the focus back in its field; Close or Escape closes it and clears the highlights;
// another document closes it too.

import { FindBar } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState } from "react";

/** The handlers of the registry's find commands. */
export type FindActions = Readonly<Record<"find.open" | "find.next" | "find.previous", () => void>>;

/** The find commands' handlers, the bar while find is open, and whether it is. */
export function useFind(
  viewer: Viewer | undefined,
  file: object | undefined,
): { actions: FindActions; bar: ReactNode | undefined; open: boolean } {
  const [open, setOpen] = useState(false);
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
      <FindBar find={viewer.find} onClose={close} focusRequest={focusRequest} />
    ) : undefined;
  return { actions, bar, open: bar !== undefined };
}
