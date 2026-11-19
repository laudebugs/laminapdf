// A key that changes whenever the edit session makes, undoes or redoes a step: what a panel that
// reads the document (the Outline tab's bookmarks, the Comments panel) reads again on.

import type { Viewer } from "@laminapdf/viewer";
import { useSyncExternalStore } from "react";

/** A key that changes whenever `viewer`'s edit session makes, undoes or redoes a step. */
export function useEditStep(viewer: Viewer): string {
  return useSyncExternalStore(
    (listener) => viewer.edits.subscribe(listener),
    () => {
      const { history } = viewer.edits.getState();
      return `${history.dropped}:${history.steps.length}:${history.position}`;
    },
  );
}
