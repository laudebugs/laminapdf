// Seeding a dialog's form each time it opens, while it renders (React's "adjusting state when a
// prop changes"), never in an effect, which would show the old values for a frame.

import { useState } from "react";

/**
 * Calls `reset` as the dialog opens, and only then, during that render: a re-render of the reader
 * while it is open keeps what the person typed or chose.
 */
export function useOnOpen(isOpen: boolean, reset: () => void): void {
  const [opened, setOpened] = useState(false);
  if (isOpen !== opened) {
    setOpened(isOpen);
    if (isOpen) reset();
  }
}
