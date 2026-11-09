// The page's network feed, installed before anything else of the app runs (main.tsx imports this
// module first), so no code on the page can capture an unwrapped `fetch` or `WebSocket` first.
// The document worker installs its own (`@laminapdf/viewer`'s worker/network.ts).

import { watchNetwork } from "@laminapdf/viewer";
import { NetworkMonitor } from "./monitor.ts";

/** Every request the page and the document worker make, for the status bar and the network panel. */
export const networkMonitor = new NetworkMonitor();

watchNetwork(
  globalThis,
  {
    request: (record) => networkMonitor.pageRequest(record),
    violation: (violation) => networkMonitor.pageViolation(violation),
  },
  "page",
);
