// Whether a service worker controls this page (C7b), as a store: the network panel lists it among
// the feeds it counts from. Its own downloads are the reader's files from this site, made when it
// installs; no page or document worker sees them, so the panel doesn't count them.

import type { ExternalStore } from "@laminapdf/ui";

/** The parts of `navigator.serviceWorker` used here. */
export interface ControlScope {
  readonly controller: unknown;
  addEventListener(type: "controllerchange", listener: () => void): void;
  removeEventListener(type: "controllerchange", listener: () => void): void;
}

/** `scope`'s control, as a store: `true` while a service worker controls the page. */
export function controlStore(scope: ControlScope | undefined): ExternalStore<boolean> {
  return {
    getState: () => scope !== undefined && scope.controller !== null,
    subscribe: (listener) => {
      scope?.addEventListener("controllerchange", listener);
      return () => scope?.removeEventListener("controllerchange", listener);
    },
  };
}

/** This page's. */
export const serviceWorkerControl = controlStore(
  typeof navigator !== "undefined" && "serviceWorker" in navigator ? navigator.serviceWorker : undefined,
);
