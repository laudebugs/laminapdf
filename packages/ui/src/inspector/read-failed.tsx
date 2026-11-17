// What the object inspector shows where a read failed (the document's summary, a tab's facts, an
// object, what refers to it): the reason and Try again, which asks for the read once more.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { laterMessage } from "../later-messages.js";

/** A failed read: its reason, and Try again. */
export function ReadFailed({ reason, onRetry }: { reason: string; onRetry: () => void }): ReactNode {
  return (
    <div className="paper-inspect-problem" role="alert">
      <span>{laterMessage("inspect.failed", { reason })}</span>
      <Button size={30} onPress={onRetry}>
        {laterMessage("inspect.retry")}
      </Button>
    </div>
  );
}
