// Whether the browser is online, as a store for React: `navigator.onLine`, updated by the
// window's `online` and `offline` events. Offline means no network at all; a site that refuses or
// a slow connection still reads as online, and the open path reports those itself. While offline
// it also looks again every few seconds: a page loaded with no network can miss the `online`
// event (Firefox does), and the notice would then outlive the outage.

import type { ExternalStore } from "@laminapdf/ui";

/** The parts of a window this uses; tests pass a fake. */
export interface OnlineScope {
  readonly navigator: { readonly onLine: boolean };
  addEventListener(type: "online" | "offline", listener: () => void): void;
  removeEventListener(type: "online" | "offline", listener: () => void): void;
  setInterval(handler: () => void, ms: number): number;
  clearInterval(id: number): void;
}

/** How often to look again while offline. */
const RECHECK_MS = 3000;

/** The connection, as a store: `true` while online. */
export function onlineStore(scope: OnlineScope): ExternalStore<boolean> {
  return {
    getState: () => scope.navigator.onLine,
    subscribe: (listener) => {
      let last = scope.navigator.onLine;
      const check = () => {
        if (scope.navigator.onLine === last) return;
        last = scope.navigator.onLine;
        listener();
      };
      scope.addEventListener("online", check);
      scope.addEventListener("offline", check);
      const timer = scope.setInterval(() => {
        if (!last) check();
      }, RECHECK_MS);
      return () => {
        scope.removeEventListener("online", check);
        scope.removeEventListener("offline", check);
        scope.clearInterval(timer);
      };
    },
  };
}
