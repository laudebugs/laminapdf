// The new tab's side of a hand-off (handoff.ts): the document another tab made for this one
// opens as soon as it arrives, and the token leaves the address. Loaded only for such a tab.

import type { Engine } from "../session.ts";
import { handoffToken, receive } from "./handoff.ts";
import { memoryFile } from "./memory-file.ts";

/** Opens the document the address's hand-off token names, once it comes. */
export function takeHandoff(engine: Engine): void {
  const token = handoffToken(location.hash);
  if (token === undefined) return;
  history.replaceState(null, "", `${location.pathname}${location.search}`);
  receive(token, {
    open: (name, bytes) => engine.reader.open(memoryFile(name, bytes)).catch(reportError),
    giveUp: () => window.close(),
  });
}
