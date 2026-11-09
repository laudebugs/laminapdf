// Find in the reader (R05): the find bar in the secondary-toolbar slot while find is open, and the
// registry's find commands on the viewer's find controller. Ctrl+F or ⌘F (`find.open`) opens the
// bar, or puts the focus back in its field; Close or Escape closes it and clears the highlights;
// another document closes it too.

import { FindBar } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState } from "react";

/** The find commands' handlers, and the bar while find is open. */
export function useFind(
  viewer: Viewer | undefined,
  file: object | undefined,
): { actions: Record<"find.open" | "find.next" | "find.previous", () => void>; bar: ReactNode | undefined } {
  const [open, setOpen] = useState(false);
  // Another document, or none: find closes.
  useEffect(() => {
    setOpen(false);
    viewer?.find.close();
  }, [file]);
  const actions = useMemo(
    () => ({
      "find.open": () => {
        if (!open) setOpen(true);
        else document.querySelector<HTMLInputElement>(".paper-find-input")?.select();
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
  const bar = open && viewer !== undefined ? <FindBar find={viewer.find} onClose={close} /> : undefined;
  return { actions, bar };
}
